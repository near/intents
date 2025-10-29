use std::borrow::Cow;

use defuse_near_utils::{UnwrapOrPanic, time::now};
use defuse_nep245::{ext_mt_core, receiver::MultiTokenReceiver};

use defuse_token_id::nep245::Nep245TokenId as TokenId;
use impl_tools::autoimpl;
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseOrValue, PromiseResult, env,
    json_types::U128, near, require, serde_json,
};

use crate::{
    Action, CreatedEvent, Error, Escrow, EscrowIntentEmit, FillAction, FixedParams, OpenAction,
    Params, Result, Storage, TransferMessage,
};

const MT_TRANSFER_GAS: Gas = Gas::from_tgas(15);

// mod old;

// TODO: lost&found?

// TODO: emit logs

// TODO: refund storage_deposits from maker/taker on received tokens
// solution: use intents.near NEP-245

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

// TODO: too large state (> ZBA limits)
// solution?: keep hashes of immutable data?
// or maybe even compare with current_account_id?

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
        CreatedEvent {
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
impl MultiTokenReceiver for Contract {
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        let (token_id, amount) = single(token_ids)
            .zip(single(amounts))
            .ok_or(Error::WrongAsset)
            .unwrap_or_panic();

        let asset = TokenId::new(env::predecessor_account_id(), token_id)
            // TODO
            .unwrap();

        let refund = self
            .on_receive(sender_id, asset, amount.0, &msg)
            .unwrap_or_panic();

        PromiseOrValue::Value(vec![U128(refund)])
    }
}

#[near]
impl Escrow for Contract {
    fn view(&self) -> &Storage {
        &self.0
    }

    // TODO: cancel_by_resolver?
    #[payable]
    fn close(&mut self) -> Promise {
        // TODO
        todo!()
        // if now() <= self.params.deadline {
        //     // TODO: what if swapped everything already?
        //     // what if more assets are about to arrive?
        //     require!(
        //         self.cancel_authority == Some(env::predecessor_account_id()),
        //         "unauthorized"
        //     );
        //     assert_one_yocto();
        // }

        // require!(!self.state.closed, "already closed");
        // self.state.closed = true;

        // // TODO: ensure src_remaining > 0

        // Self::send(
        //     self.src_asset.clone(),
        //     // TODO: refund_to?
        //     self.maker.clone(),
        //     self.src_remaining,
        // )
        // .and(Promise::new(env::current_account_id()).delete_account(self.maker.clone()))
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

        self.state.src_remaining = self
            .state
            .src_remaining
            .checked_add(amount)
            .ok_or(Error::IntegerOverflow)?;

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
        // TODO: taker whitelist

        let (taker_src_amount, mut maker_dst_amount) = {
            let want_src_amount = self
                .params
                .price
                .src_amount(dst_amount)
                .ok_or(Error::IntegerOverflow)?;
            // TODO: fees
            if want_src_amount < self.state.src_remaining {
                if !fixed.partial_fills_allowed {
                    return Err(Error::PartialFillsNotAllowed);
                }
                (want_src_amount, dst_amount)
            } else {
                (
                    self.state.src_remaining,
                    self.params
                        .price
                        // TODO: rounding inside?
                        .dst_amount(self.state.src_remaining)
                        .ok_or(Error::IntegerOverflow)?,
                )
            }
        };

        // TODO: check taker_src_amount != 0 && maker_dst_amount != 0
        self.state.src_remaining -= taker_src_amount;
        let refund = dst_amount - maker_dst_amount;

        // send to taker
        let _ = Self::send(
            fixed.src_asset,
            msg.receiver_id.unwrap_or(sender_id),
            taker_src_amount,
            None,
        );

        // send fees
        for (fee_collector, fee) in &fixed.fees {
            let fee_amount = fee.fee_ceil(maker_dst_amount);
            maker_dst_amount = maker_dst_amount
                .checked_sub(fee_amount)
                .ok_or(Error::IntegerOverflow)?;
            let _ = Self::send(
                fixed.dst_asset.clone(),
                fee_collector.clone(),
                fee_amount,
                None,
            );
        }

        let _ = Self::send(
            fixed.dst_asset,
            fixed
                .maker_dst_receiver_id
                .as_ref()
                .unwrap_or(&fixed.maker)
                .clone(),
            maker_dst_amount,
            None,
        );

        Ok(refund)
    }

    fn send(asset: TokenId, receiver_id: AccountId, amount: u128, msg: Option<String>) -> Promise {
        // TODO: msg for *_transfer_call()?
        let (contract_id, token_id) = asset.clone().into_contract_id_and_mt_token_id();

        let p = ext_mt_core::ext(contract_id)
            // TODO: are we sure we have that???
            .with_attached_deposit(NearToken::from_yoctonear(1));
        if let Some(msg) = msg {
            todo!()
        } else {
            p.with_static_gas(MT_TRANSFER_GAS).mt_transfer(
                receiver_id,
                token_id,
                U128(amount),
                None,
                None,
            )
        }
    }
}

#[near]
impl Contract {
    #[private]
    // TODO: was it dst or src (i.e. close)?
    pub fn resolve_maker(&mut self, amount: U128, is_call: bool) {
        let used = match env::promise_result(0) {
            PromiseResult::Successful(value) => {
                if is_call {
                    // `ft_transfer_call` returns successfully transferred amount
                    serde_json::from_slice::<U128>(&value)
                        .unwrap_or_default()
                        .0
                        .min(amount.0)
                } else if value.is_empty() {
                    // `ft_transfer` returns empty result on success
                    amount.0
                } else {
                    0
                }
            }
            PromiseResult::Failed => {
                if is_call {
                    // do not refund on failed `ft_transfer_call` due to
                    // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                    // read result of `ft_on_transfer` due to insufficient gas
                    amount.0
                } else {
                    0
                }
            }
        };

        let refund = amount.0.saturating_sub(used);
        if refund > 0 {
            // TODO
        }
    }
}

fn single<T>(v: Vec<T>) -> Option<T> {
    let [a] = v.try_into().ok()?;
    Some(a)
}
