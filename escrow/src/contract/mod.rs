mod cleanup;
mod return_value;
mod tokens;
mod utils;

use std::{borrow::Cow, mem};

use defuse_near_utils::{MaybePromise, PromiseExt, UnwrapOrPanic, UnwrapOrPanicError};
use defuse_token_id::{TokenId, TokenIdType};

use near_sdk::{
    AccountId, Gas, PanicOnDefault, Promise, PromiseOrValue, env, json_types::U128, near, require,
    serde_json,
};
use strum::IntoDiscriminant;

use crate::{
    Action, AddSrcEvent, CreateEvent, Error, Escrow, EscrowEvent, EscrowIntentEmit, FillAction,
    FillEvent, FixedParams, OpenAction, Params, Result, Storage, TransferMessage,
    contract::tokens::SentAsset,
};

use self::{cleanup::CleanupGuard, return_value::ReturnValueExt, tokens::Sendable};

// QUESTIONS:
// * cancel by 2-of-2 multisig: user + SolverBus?
//   * why not 1-of-2 by SolverBus?
//

// governor: partial release

// TODO: add support for custom ".on_settled()" hooks?

// TODO: streaming swaps:
// * cancel of long streaming swaps?
// solution: time-lock (i.e. "delayed" canceling)
// + solver can confirm that he acknoliged the cancel, so it's a multisig 2-of-2 for immediate cancellation

// TODO: solver: create_subescrow and lock NEAR on it
// TODO: refund locked NEAR back to taker if closed Ok, otherwise...?

// TODO: coinsidence of wants?

// TODO: custom_resolve()

// ratio is not a good approach, since adding more maker_tokens
// would keep the ratio same, rather than decreasing it
// BUT: maker doesn't want to blindly add assets, he wants to increase to a specific ratio
// so, he should send maker_asset and forward target_price, the rest will be refunded to him,
// since there could have been in-flight taker assets coming to the escrow
// TODO: remaining_amount?
// pub taker_amount: u128,

// TODO: recovery() method with 0 src_remaining or without msg
#[near(contract_state)] // TODO: (key = "")
#[derive(Debug, PanicOnDefault)]
pub struct Contract(Option<Storage>);

#[near]
impl Contract {
    #[init]
    pub fn escrow_init(fixed: &FixedParams, params: Params) -> Self {
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

        Self(Some(s))
    }
}

#[near]
impl Escrow for Contract {
    fn escrow_view(&self) -> &Storage {
        self.try_as_alive().unwrap_or_panic()
    }

    // TODO: cancel_by_resolver?
    #[payable]
    fn escrow_close(&mut self, fixed_params: FixedParams) -> PromiseOrValue<bool> {
        self.cleanup_guard().close(fixed_params).unwrap_or_panic()
    }

    fn escrow_lost_found(&mut self, fixed_params: FixedParams) -> PromiseOrValue<bool> {
        self.cleanup_guard()
            .lost_found(fixed_params)
            .unwrap_or_panic()
    }
}

impl Contract {
    const fn cleanup_guard(&mut self) -> CleanupGuard<'_> {
        CleanupGuard::new(self)
    }

    const fn as_alive(&self) -> Option<&Storage> {
        self.0.as_ref()
    }

    fn try_as_alive(&self) -> Result<&Storage> {
        self.as_alive().ok_or(Error::CleanupInProgress)
    }

    fn send_with_sent(
        token: TokenId,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> (SentAsset, Promise) {
        (
            SentAsset {
                asset_type: token.discriminant(),
                amount,
                is_call: msg.is_some(),
            },
            token.send(receiver_id, amount, memo, msg, min_gas, unused_gas),
        )
    }
}

impl CleanupGuard<'_> {
    pub fn on_receive(
        &mut self,
        sender_id: AccountId,
        token_id: TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<PromiseOrValue<u128>> {
        self.try_get_mut()?
            .on_receive(sender_id, token_id, amount, msg)
    }

    fn close(&mut self, fixed: FixedParams) -> Result<PromiseOrValue<bool>> {
        let this = self.try_get_mut()?;
        this.verify(&fixed)?;

        // TODO: authority
        if !(this.params.deadline.has_expired()
            || this.state.maker_src_remaining == 0 && fixed.maker == env::predecessor_account_id()
            || fixed.taker_whitelist == [env::predecessor_account_id()].into())
        {
            // TODO: require 1yN for permissioned

            // TODO: allow to close permissionlessly if src_remaining == 0
            // TODO: what if more assets are about to arrive? allow only for maker
            // TODO: allow force close by authority (`force: bool` param?)
            // TODO: different error

            // TODO:
            return Err(Error::DeadlineNotExpired);
        }

        let just_closed = !mem::replace(&mut this.state.closed, true);
        // TODO: what if already closed? maybe not allow closing twice?
        if just_closed {
            // TODO: allow retries
            // TODO: enrich event
            // TODO: do not emit event twice
            EscrowEvent::Close.emit();
        }

        // TODO: this will call try_as_alive_mut() and verify(fixed) twice
        self.lost_found(fixed)
    }

    fn lost_found(&mut self, fixed: FixedParams) -> Result<PromiseOrValue<bool>> {
        let this = self.try_get_mut()?;
        this.verify(&fixed)?;

        let (sent_src, send_src_p) = this
            .state
            .closed
            .then(|| mem::take(&mut this.state.maker_src_remaining))
            .filter(|a| *a > 0)
            .map(|amount| {
                Contract::send_with_sent(
                    fixed.src_asset,
                    fixed
                        .refund_src_to
                        .receiver_id
                        .unwrap_or_else(|| fixed.maker.clone()),
                    amount,
                    fixed.refund_src_to.memo,
                    fixed.refund_src_to.msg,
                    fixed.refund_src_to.min_gas,
                    true, // unused gas
                )
            })
            .unzip();

        // TODO: maybe don't retry dst_lost here?
        let (sent_dst, send_dst_p) = Some(mem::take(&mut this.state.maker_dst_lost))
            .filter(|a| *a > 0)
            .map(|amount| {
                Contract::send_with_sent(
                    fixed.dst_asset,
                    fixed.receive_dst_to.receiver_id.unwrap_or(fixed.maker),
                    amount,
                    fixed.receive_dst_to.memo,
                    fixed.receive_dst_to.msg,
                    fixed.receive_dst_to.min_gas,
                    true, // unused gas
                )
            })
            .unzip();

        let Some(send) = send_src_p
            .into_iter()
            .chain(send_dst_p)
            .reduce(Promise::and)
        else {
            return Ok(PromiseOrValue::Value(self.maybe_cleanup(None).is_some()));
        };

        Ok(send
            .then(
                this.callback()
                    .with_static_gas(Contract::ESCROW_RESOLVE_TRANSFERS_GAS)
                    .with_unused_gas_weight(0)
                    .escrow_resolve_transfers(sent_src, sent_dst, env::predecessor_account_id()),
            )
            .into())
    }

    fn resolve_transfers(
        &mut self,
        maker_src: Option<SentAsset>,
        maker_dst: Option<SentAsset>,
        beneficiary_id: AccountId,
    ) -> Result<bool> {
        let this = self.try_get_mut()?;
        this.on_callback();

        for (result_idx, (sent, lost)) in maker_src
            .zip(Some(&mut this.state.maker_src_remaining))
            .into_iter()
            .chain(maker_dst.zip(Some(&mut this.state.maker_dst_lost)))
            .enumerate()
        {
            let refund =
                sent.resolve_refund(result_idx.try_into().unwrap_or_else(|_| unreachable!()));

            // TODO: emit event if non-zero refund?
            *lost = lost.checked_add(refund).ok_or(Error::IntegerOverflow)?;
        }

        Ok(self.maybe_cleanup(beneficiary_id).is_some())
    }
}

impl Storage {
    /// Returns refund amount
    fn on_receive(
        &mut self,
        // TODO: it could have been EscrowFactory who forwarded funds to us
        sender_id: AccountId,
        token_id: TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<PromiseOrValue<u128>> {
        // TODO: check amount non-zero
        if self.state.closed || self.params.deadline.has_expired() {
            // TODO: utilize for our needs, refund after being closed or expired?
            // TODO: what if maker wants to reopen and prolongate deadline?
            return Err(Error::Closed);
        }

        let msg: TransferMessage = serde_json::from_str(msg)?;
        self.verify(&msg.fixed_params)?;

        match msg.action {
            Action::Open(open) if token_id == msg.fixed_params.src_asset => {
                self.on_open(msg.fixed_params, sender_id, amount, open)
            }
            Action::Fill(fill) if token_id == msg.fixed_params.dst_asset => {
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
    ) -> Result<PromiseOrValue<u128>> {
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

        Ok(PromiseOrValue::Value(0))
    }

    fn on_fill(
        &mut self,
        fixed: FixedParams,
        sender_id: AccountId,
        dst_amount: u128,
        msg: FillAction,
    ) -> Result<PromiseOrValue<u128>> {
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
        let send_fees = {
            let mut sends: Option<Promise> = None;
            let dst_fees_collected = fixed
                .fees
                .iter()
                .map(|(fee_collector, fee)| {
                    let fee_amount = fee.fee_ceil(dst_used);
                    if fee_amount > 0 {
                        maker_dst_amount = maker_dst_amount
                            .checked_sub(fee_amount)
                            .ok_or(Error::ExcessiveFees)?;

                        sends = Some(sends.take().and_or(fixed.dst_asset.clone().send(
                            fee_collector.clone(),
                            fee_amount,
                            Some("fee".to_string()),
                            None, // TODO: msg for fee_collectors?
                            None,
                            false, // no unused gas
                        )));
                    }

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

            sends
        };

        if taker_src_amount == 0 || maker_dst_amount == 0 {
            // TODO: maybe check earlier?
            return Err(Error::InsufficientAmount);
        }

        // send to maker
        let (maker_dst, maker_dst_p) = Contract::send_with_sent(
            fixed.dst_asset,
            fixed.receive_dst_to.receiver_id.unwrap_or(fixed.maker),
            maker_dst_amount,
            fixed.receive_dst_to.memo,
            fixed.receive_dst_to.msg,
            fixed.receive_dst_to.min_gas,
            true, // unused gas
        );

        // TODO: lost&found?
        Ok(maker_dst_p
            // send to taker
            .and(
                fixed.src_asset.send(
                    msg.receive_src_to
                        .receiver_id
                        .unwrap_or_else(|| sender_id.clone()),
                    taker_src_amount,
                    msg.receive_src_to.memo,
                    msg.receive_src_to.msg,
                    msg.receive_src_to.min_gas,
                    true, // unused gas
                ),
            )
            .maybe_and(send_fees)
            .then({
                let refund_value = match &maker_dst.asset_type {
                    #[cfg(feature = "nep141")]
                    TokenIdType::Nep141 => serde_json::to_vec(&U128(refund)),
                    #[cfg(feature = "nep245")]
                    TokenIdType::Nep245 => serde_json::to_vec(&[U128(refund)]),
                }?;

                self.callback()
                    .with_static_gas(Contract::ESCROW_RESOLVE_TRANSFERS_GAS)
                    .with_unused_gas_weight(0)
                    .escrow_resolve_transfers(
                        None,
                        Some(maker_dst),
                        // TODO: beneficiary_id
                        sender_id,
                    )
                    .return_value(refund_value)
            })
            .into())
    }

    fn callback(&mut self) -> ContractExt {
        self.state.callbacks_in_flight = self
            .state
            .callbacks_in_flight
            .checked_add(1)
            .ok_or("too many callbacks in flight")
            .unwrap_or_panic_static_str();
        Contract::ext(env::current_account_id())
    }

    fn on_callback(&mut self) {
        self.state.callbacks_in_flight = self
            .state
            .callbacks_in_flight
            .checked_sub(1)
            .ok_or("unregistered callback")
            .unwrap_or_panic_static_str();
    }
}

#[near]
impl Contract {
    const ESCROW_RESOLVE_TRANSFERS_GAS: Gas = Gas::from_tgas(10);

    #[private]
    pub fn escrow_resolve_transfers(
        &mut self,
        maker_src: Option<SentAsset>,
        maker_dst: Option<SentAsset>,
        beneficiary_id: AccountId,
    ) -> bool {
        self.cleanup_guard()
            .resolve_transfers(maker_src, maker_dst, beneficiary_id)
            .unwrap_or_panic()
    }
}
