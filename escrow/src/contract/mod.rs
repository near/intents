mod tokens;
mod utils;

use std::{borrow::Cow, mem};

use defuse_near_utils::time::now;
use defuse_token_id::TokenId;
use impl_tools::autoimpl;
use near_sdk::{
    AccountId, Gas, PanicOnDefault, Promise, PromiseOrValue, PromiseResult, env, json_types::U128,
    near, require, serde_json,
};

use crate::{
    Action, AddSrcEvent, CreateEvent, Error, Escrow, EscrowEvent, EscrowIntentEmit, FillAction,
    FillEvent, FixedParams, OpenAction, Params, Result, Storage, TransferMessage,
};

use self::tokens::Sender;

// mod old;

// TODO: lost&found?

// TODO: coinsidence of wants?

// TODO: custom_resolve()

// ratio is not a good approach, since adding more maker_tokens
// would keep the ratio same, rather than decreasing it
// BUT: maker doesn't want to blindly add assets, he wants to increase to a specific ratio
// so, he should send maker_asset and forward target_price, the rest will be refunded to him,
// since there could have been in-flight taker assets coming to the escrow
// TODO: remaining_amount?
// pub taker_amount: u128,

// TODO: versioned account state?
// TODO: recovery() method with 0 src_remaining

// TODO: keep number of pending promises
#[near(contract_state)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[derive(Debug, PanicOnDefault)]
pub struct Contract(Storage);

#[near]
impl Contract {
    #[init]
    pub fn new(fixed: &FixedParams, params: Params) -> Self {
        CreateEvent {
            fixed: Cow::Borrowed(&fixed),
            params: Cow::Borrowed(&params),
        }
        .emit();

        let s = Storage::new(fixed, params);

        // just for the safety
        require!(
            env::current_account_id() == s.derive_account_id(env::predecessor_account_id()),
            "wrong params or factory"
        );

        Self(s)
    }
}

#[near]
impl Escrow for Contract {
    fn view(&self) -> &Storage {
        &self.0
    }

    // TODO: cancel_by_resolver?
    #[payable]
    fn close(&mut self, fixed_params: FixedParams) -> PromiseOrValue<U128> {
        // TODO
        if now() <= self.params.deadline {
            // TODO: allow to close permissionlessly if src_remaining == 0

            // TODO: what if swapped everything already?
            // what if more assets are about to arrive?
            // require!(
            //     self.cancel_authority == Some(env::predecessor_account_id()),
            //     "unauthorized"
            // );
            // assert_one_yocto();
        }

        // TODO: allow retries
        require!(
            !self.state.closed || self.state.maker_src_remaining > 0,
            "already closed or closing"
        );
        self.state.closed = true;
        // TODO: enrich event
        EscrowEvent::Close.emit();

        let refund_to = fixed_params
            .refund_src_to
            .receiver_id
            .unwrap_or(fixed_params.maker);

        if self.state.maker_src_remaining == 0 {
            let _ = Promise::new(env::current_account_id()).delete_account(refund_to);

            return PromiseOrValue::Value(U128(0));
        }

        let refund = mem::take(&mut self.state.maker_src_remaining);

        let is_call = fixed_params.refund_src_to.msg.is_some();
        Self::send(
            fixed_params.src_asset,
            refund_to.clone(),
            refund,
            fixed_params.refund_src_to.memo,
            fixed_params.refund_src_to.msg,
            fixed_params.refund_src_to.min_gas,
        )
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(Self::RESOLVE_MAKER_GAS)
                .with_unused_gas_weight(0)
                .resolve_maker(U128(refund), is_call, refund_to),
        )
        .into()
    }
}

impl Contract {
    /// Returns refund amount
    fn on_receive(
        &mut self,
        sender_id: AccountId,
        asset: TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<u128> {
        // TODO: check amount non-zero
        if self.state.closed || now() > self.params.deadline {
            // TODO: utilize for our needs, refund after being closed or expired?
            return Err(Error::Closed);
        }

        let msg: TransferMessage = serde_json::from_str(msg)?;
        self.verify(&msg.fixed_params)?;

        match msg.action {
            Action::Open(open) if asset == msg.fixed_params.src_asset => {
                self.on_open(msg.fixed_params, sender_id, amount, open)
            }
            Action::Fill(fill) if asset == msg.fixed_params.dst_asset => {
                self.on_fill(msg.fixed_params, sender_id, amount, fill)
            }
            _ => Err(Error::WrongAsset),
        }
    }

    fn on_open(
        &mut self,
        fixed: FixedParams,
        sender_id: AccountId,
        amount: u128,
        msg: OpenAction,
    ) -> Result<u128> {
        if sender_id != fixed.maker {
            return Err(Error::Unauthorized);
        }

        self.state.maker_src_remaining = self
            .state
            .maker_src_remaining
            .checked_add(amount)
            .ok_or(Error::IntegerOverflow)?;

        AddSrcEvent {
            maker: sender_id,
            src_amount_added: amount,
            src_remaining: self.state.maker_src_remaining,
        }
        .emit();

        // TODO: allow for extended deadline prolongation in msg?
        // TODO: but how can we verify sender_id to allow for that?

        if let Some(new_price) = msg.new_price {
            if new_price < self.params.price {
                // TODO: or ignore?
                return Err(Error::LowerPrice);
            }
            self.params.price = new_price;
        }

        Ok(0)
    }

    fn on_fill(
        &mut self,
        fixed: FixedParams,
        sender_id: AccountId,
        dst_amount: u128,
        msg: FillAction,
    ) -> Result<u128> {
        if !(fixed.taker_whitelist.is_empty() || fixed.taker_whitelist.contains(&sender_id)) {
            // TODO: or authority?
            return Err(Error::Unauthorized);
        }

        let (taker_src_amount, dst_used) = {
            let want_src_amount = self
                .params
                .price
                .src_amount(dst_amount)
                .ok_or(Error::IntegerOverflow)?;
            // TODO: what if zero?
            if want_src_amount < self.state.maker_src_remaining {
                if !fixed.partial_fills_allowed {
                    return Err(Error::PartialFillsNotAllowed);
                }
                (want_src_amount, dst_amount)
            } else {
                (
                    self.state.maker_src_remaining,
                    self.params
                        .price
                        // TODO: rounding inside?
                        .dst_amount(self.state.maker_src_remaining)
                        .ok_or(Error::IntegerOverflow)?,
                )
            }
        };

        self.state.maker_src_remaining -= taker_src_amount;
        let refund = dst_amount - dst_used;

        let mut maker_dst_amount = dst_used;

        // collect, subtract and send fees
        {
            let dst_fees_collected = fixed
                .fees
                .iter()
                .map(|(fee_collector, fee)| {
                    let fee_amount = fee.fee_ceil(dst_used);

                    maker_dst_amount = maker_dst_amount
                        .checked_sub(fee_amount)
                        .ok_or(Error::ExcessiveFees)?;

                    let _ = Self::send(
                        fixed.dst_asset.clone(),
                        fee_collector.clone(),
                        fee_amount,
                        Some("fee".to_string()),
                        None,
                        None,
                    );
                    Ok((fee_collector.into(), fee_amount))
                })
                .collect::<Result<_>>()?;

            FillEvent {
                taker: Cow::Borrowed(&sender_id),
                src_amount: taker_src_amount,
                dst_amount,
                taker_receiver_id: msg.receive_src_to.receiver_id.as_deref().map(Cow::Borrowed),
                dst_fees_collected,
            }
            .emit();
        }

        if taker_src_amount == 0 || maker_dst_amount == 0 {
            // TODO: maybe check earlier?
            return Err(Error::InsufficientAmount);
        }

        // send to taker
        let _ = Self::send(
            fixed.src_asset,
            msg.receive_src_to.receiver_id.unwrap_or(sender_id),
            taker_src_amount,
            msg.receive_src_to.memo,
            msg.receive_src_to.msg,
            msg.receive_src_to.min_gas,
        );

        // TODO: lost&found?
        // send to maker
        let _ = Self::send(
            fixed.dst_asset,
            fixed
                .receive_dst_to
                .receiver_id
                .as_ref()
                .unwrap_or(&fixed.maker)
                .clone(),
            maker_dst_amount, // TODO: check non-zero
            fixed.receive_dst_to.memo,
            fixed.receive_dst_to.msg,
            fixed.receive_dst_to.min_gas,
        );

        Ok(refund)
    }

    // TODO: rename
    fn maybe_cleanup(&mut self, beneficiary_id: AccountId) -> Option<Promise> {
        if self.params.deadline > now() {
            // TODO: are we sure?
            self.state.closed = true;
        }

        if !self.state.closed || self.state.maker_src_remaining > 0 {
            return None;
        }

        // NOTE: Unfortunately, we can't refund `storage_deposit`s on src and dst
        // tokens back to maker or any other beneficiary, since
        // `storage_unregister()` internally detaches transfer Promise, so we
        // don't know when it arrives and can't schedule the cleanup afterwards.
        Some(Promise::new(env::current_account_id()).delete_account(beneficiary_id))
    }
}

#[near]
impl Contract {
    const RESOLVE_MAKER_GAS: Gas = Gas::from_tgas(10);

    #[private]
    // TODO: was it dst or src (i.e. close)?
    pub fn resolve_maker(
        &mut self,
        amount: U128,
        is_call: bool,
        beneficiary_id: AccountId,
    ) -> U128 {
        let used = resolve_mt_transfer(0, amount.0, is_call);

        let refund = amount.0.saturating_sub(used);
        if refund > 0 {
            self.state.maker_src_remaining = self
                .state
                .maker_src_remaining
                .checked_add(refund)
                // TODO: is it?
                .unwrap_or_else(|| unreachable!());
            // TODO: emit event
        }

        // detach promise
        let _ = self.maybe_cleanup(beneficiary_id);

        U128(refund)
        // TODO: maybe delete self?
    }
}

// Returns actually transferred amount of a single MT token.
fn resolve_mt_transfer(result_idx: u64, amount: u128, is_call: bool) -> u128 {
    match env::promise_result(result_idx) {
        PromiseResult::Successful(value) => {
            if is_call {
                // `mt_transfer_call` returns successfully transferred amounts
                serde_json::from_slice::<[U128; 1]>(&value).unwrap_or_default()[0]
                    .0
                    .min(amount)
            } else if value.is_empty() {
                // `mt_transfer` returns empty result on success
                amount
            } else {
                0
            }
        }
        PromiseResult::Failed => {
            if is_call {
                // do not refund on failed `mt_transfer_call` due to
                // NEP-141 vulnerability: `mt_resolve_transfer` fails to
                // read result of `mt_on_transfer` due to insufficient gas
                amount
            } else {
                0
            }
        }
    }
}
