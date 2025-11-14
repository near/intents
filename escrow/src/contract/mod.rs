#[cfg(feature = "auth_call")]
mod auth_call;
mod cleanup;
mod resolve;
mod return_value;
mod tokens;
mod utils;

use std::{borrow::Cow, mem};

use defuse_near_utils::{MaybePromise, PromiseExt, UnwrapOrPanic};
use defuse_token_id::TokenId;

use near_sdk::{AccountId, PanicOnDefault, Promise, PromiseOrValue, env, near, require};

use crate::{
    AddSrcEvent, Error, Escrow, EscrowIntentEmit, Event, FillEvent, Result,
    state::{Params, State, Storage},
    tokens::{FillAction, OpenAction, TransferAction},
};

use self::{
    return_value::ReturnValueExt,
    tokens::{Sendable, TokenIdTypeExt},
};

// TODO: add support for NFTs

// TODO: add support for custom ".on_settled()" hooks?

// TODO: streaming swaps:
// * cancel of long streaming swaps?
// solution: time-lock (i.e. "delayed" canceling)
// + solver can confirm that he acknoliged the cancel, so it's a multisig 2-of-2 for immediate cancellation

// TODO: solver: create_subescrow and lock NEAR on it
// TODO: refund locked NEAR back to taker if closed Ok, otherwise...?

// TODO: coinsidence of wants?
// user1: locked 1 BTC in escrow for swap to 100k USDC
// user2: sends RFQ to SolverBus to swap 10k USDC to BTC
// SolverBus sends him address of escrow contract,
// user2 signs "transfer" intent:
// `{
//   "receiver_id": "0s123...abc" // address of escrow
//   "token": "<USDC ADDRESS>",
//   "amount": "10k",
//   "msg": "FILL MSG + SOLVER_BUS SIGNATURE",
// }`
// user2 transfers to "solver-bus-proxy.near" escrow, tries to fill, if fail -> refund
// OR: we can have intermediary contract to refund to ANOTHER ESCROW to reduce failure rate
//
// if we make solvers to be MMs, then solver-bus-proxy.near can
// implement CLOB

// TODO: lending
// solver -> escrow::mt_on_transfer(sender_id, token_id, amount, msg)
//        * msg: loan
//
//        -> escrow_loan:
//

// TODO: recovery() method with 0 src_remaining or without msg
#[near(contract_state)] // TODO: (key = "")
#[derive(Debug, PanicOnDefault)]
pub struct Contract(Option<Storage>);

#[near]
impl Contract {
    #[init]
    pub fn escrow_init(params: &Params) -> Self {
        Event::Create(Cow::Borrowed(&params)).emit();

        let s = Storage::new(params).unwrap_or_panic();

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
        self.try_as_alive()
            // if cleanup is in progress, the contract will be
            // soon deleted anyway, so it's ok to panic here
            .unwrap_or_panic()
    }

    #[payable]
    fn escrow_close(&mut self, params: Params) -> PromiseOrValue<bool> {
        self.close(env::predecessor_account_id(), params)
            .unwrap_or_panic()
    }

    fn escrow_lost_found(&mut self, params: Params) -> PromiseOrValue<bool> {
        self.lost_found(params).unwrap_or_panic()
    }
}

impl Contract {
    fn close(
        &mut self,
        signer_id: AccountId,
        fixed_params: Params,
    ) -> Result<PromiseOrValue<bool>> {
        let mut guard = self.cleanup_guard();

        let this = guard.try_as_alive_mut()?.verify_mut(&fixed_params)?;

        Ok(
            if let Some(promise) = this.close(signer_id, fixed_params)? {
                PromiseOrValue::Promise(promise)
            } else {
                PromiseOrValue::Value(guard.maybe_cleanup().is_some())
            },
        )
    }

    fn lost_found(&mut self, fixed_params: Params) -> Result<PromiseOrValue<bool>> {
        let mut guard = self.cleanup_guard();

        let this = guard.try_as_alive_mut()?.verify_mut(&fixed_params)?;

        Ok(if let Some(promise) = this.lost_found(fixed_params)? {
            PromiseOrValue::Promise(promise)
        } else {
            PromiseOrValue::Value(guard.maybe_cleanup().is_some())
        })
    }
}

impl State {
    /// Returns refund amount
    fn on_receive(
        &mut self,
        fixed: Params,
        // TODO: it could have been EscrowFactory who forwarded funds to us
        sender_id: AccountId,
        token_id: TokenId,
        amount: u128,
        action: TransferAction,
    ) -> Result<PromiseOrValue<u128>> {
        if self.closed || self.deadline.has_expired() {
            return Err(Error::Closed);
        }

        match action {
            TransferAction::Open(open) if token_id == fixed.src_token => {
                self.on_open(fixed, sender_id, amount, open)
            }
            TransferAction::Fill(fill) if token_id == fixed.dst_token => {
                self.on_fill(fixed, sender_id, amount, fill)
            }
            _ => Err(Error::WrongToken),
        }
    }

    fn on_open(
        &mut self,
        fixed: Params,
        sender_id: AccountId,
        amount: u128,
        msg: OpenAction,
    ) -> Result<PromiseOrValue<u128>> {
        if sender_id != fixed.maker {
            return Err(Error::Unauthorized);
        }

        self.maker_src_remaining = self
            .maker_src_remaining
            .checked_add(amount)
            .ok_or(Error::IntegerOverflow)?;

        AddSrcEvent {
            maker: sender_id,
            src_amount_added: amount,
            src_remaining: self.maker_src_remaining,
        }
        .emit();

        // TODO: allow for extended deadline prolongation in msg?
        // TODO: but how can we verify sender_id to allow for that?
        // if let Some(new_price) = msg.new_price {
        //     if new_price < self.params.price {
        //         // TODO: or ignore?
        //         return Err(Error::LowerPrice);
        //     }
        //     self.params.price = new_price;
        // }

        Ok(PromiseOrValue::Value(0))
    }

    fn on_fill(
        &mut self,
        fixed: Params,
        sender_id: AccountId,
        dst_amount: u128,
        msg: FillAction,
    ) -> Result<PromiseOrValue<u128>> {
        if !(fixed.taker_whitelist.is_empty() || fixed.taker_whitelist.contains(&sender_id)) {
            // TODO: or authority?
            return Err(Error::Unauthorized);
        }

        let (taker_src_amount, dst_used) = {
            let want_src_amount = fixed
                .price
                .src_amount(dst_amount)
                .ok_or(Error::IntegerOverflow)?;
            // TODO: what if zero?
            if want_src_amount < self.maker_src_remaining {
                if !fixed.partial_fills_allowed {
                    return Err(Error::PartialFillsNotAllowed);
                }
                (want_src_amount, dst_amount)
            } else {
                (
                    self.maker_src_remaining,
                    fixed
                        .price
                        // TODO: rounding inside?
                        .dst_amount(self.maker_src_remaining)
                        .ok_or(Error::IntegerOverflow)?,
                )
            }
        };

        self.maker_src_remaining -= taker_src_amount;
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

                        let send = fixed.dst_token.clone().send(
                            fee_collector.clone(),
                            fee_amount,
                            Some("fee".to_string()),
                            None,
                            None,
                            false, // no unused gas
                        );

                        sends = Some(sends.take().and_or(send));
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
        let (maker_dst, maker_dst_p) = fixed.dst_token.send_for_resolve(
            fixed.receive_dst_to.receiver_id.unwrap_or(fixed.maker),
            maker_dst_amount,
            fixed.receive_dst_to.memo,
            fixed.receive_dst_to.msg,
            fixed.receive_dst_to.min_gas,
            true, // unused gas
        );

        Ok(maker_dst_p
            // send to taker
            .and(
                fixed.src_token.send(
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
            .then(
                self.callback()
                    .with_static_gas(Contract::ESCROW_RESOLVE_TRANSFERS_GAS)
                    .with_unused_gas_weight(0)
                    .escrow_resolve_transfers(None, Some(maker_dst))
                    .return_value(maker_dst.token_type.refund_value(refund)?),
            )
            .into())
    }

    fn close(&mut self, signer_id: AccountId, fixed: Params) -> Result<Option<Promise>> {
        // TODO: authority
        if !(self.closed
            || self.deadline.has_expired()
            || self.maker_src_remaining == 0 && signer_id == fixed.maker
            || fixed.taker_whitelist.len() == 1 && fixed.taker_whitelist.contains(&signer_id))
        {
            // TODO: require 1yN for permissioned

            // TODO: allow to close permissionlessly if src_remaining == 0
            // TODO: what if more assets are about to arrive? allow only for maker
            // TODO: allow force close by authority (`force: bool` param?)
            // TODO: different error

            // TODO:
            return Err(Error::DeadlineNotExpired);
        }

        let just_closed = !mem::replace(&mut self.closed, true);
        // TODO: what if already closed? maybe not allow closing twice?
        if just_closed {
            // TODO: allow retries
            // TODO: enrich event
            Event::Close.emit();
        }

        self.lost_found(fixed)
    }

    fn lost_found(&mut self, fixed: Params) -> Result<Option<Promise>> {
        let (sent_src, send_src_p) = self
            .closed
            .then(|| mem::take(&mut self.maker_src_remaining))
            .filter(|a| *a > 0)
            .map(|amount| {
                fixed.src_token.send_for_resolve(
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

        let (sent_dst, send_dst_p) = Some(mem::take(&mut self.maker_dst_lost))
            .filter(|a| *a > 0)
            .map(|amount| {
                fixed.dst_token.send_for_resolve(
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
            return Ok(None);
        };

        Ok(send
            .then(
                self.callback()
                    .with_static_gas(Contract::ESCROW_RESOLVE_TRANSFERS_GAS)
                    .with_unused_gas_weight(0)
                    .escrow_resolve_transfers(sent_src, sent_dst),
            )
            .into())
    }
}
