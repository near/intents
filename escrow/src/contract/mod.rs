#[cfg(feature = "auth_call")]
mod auth_call;
mod cleanup;
mod resolve;
mod return_value;
mod tokens;
mod utils;

use std::{borrow::Cow, collections::BTreeMap, mem};

use defuse_near_utils::{MaybePromise, PromiseExt, UnwrapOrPanic};
use defuse_token_id::TokenId;

use near_sdk::{
    AccountId, AccountIdRef, PanicOnDefault, Promise, PromiseOrValue, env, near, require,
};

use crate::{
    Error, Escrow, Result,
    event::{AddSrcEvent, CloseReason, EscrowIntentEmit, Event, FillEvent, ProtocolFeesCollected},
    state::{Params, State, Storage},
    tokens::{FillAction, TransferAction},
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
    fn close(&mut self, signer_id: AccountId, params: Params) -> Result<PromiseOrValue<bool>> {
        let mut guard = self.cleanup_guard();

        let state = guard.try_as_alive_mut()?.verify_mut(&params)?;

        Ok(if let Some(promise) = state.close(signer_id, params)? {
            PromiseOrValue::Promise(promise)
        } else {
            PromiseOrValue::Value(guard.maybe_cleanup().is_some())
        })
    }

    fn lost_found(&mut self, params: Params) -> Result<PromiseOrValue<bool>> {
        let mut guard = self.cleanup_guard();

        let this = guard.try_as_alive_mut()?.verify_mut(&params)?;

        Ok(if let Some(promise) = this.lost_found(params)? {
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
        params: Params,
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
            TransferAction::Open if token_id == params.src_token => {
                self.on_open(params, sender_id, amount)
            }
            TransferAction::Fill(fill) if token_id == params.dst_token => {
                self.on_fill(params, sender_id, amount, fill)
            }
            _ => Err(Error::WrongToken),
        }
    }

    fn on_open(
        &mut self,
        params: Params,
        sender_id: AccountId,
        amount: u128,
    ) -> Result<PromiseOrValue<u128>> {
        if sender_id != params.maker {
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

        Ok(PromiseOrValue::Value(0))
    }

    fn on_fill(
        &mut self,
        params: Params,
        sender_id: AccountId,
        taker_dst_in: u128,
        msg: FillAction,
    ) -> Result<PromiseOrValue<u128>> {
        if !(params.taker_whitelist.is_empty() || params.taker_whitelist.contains(&sender_id)) {
            return Err(Error::Unauthorized);
        }

        if msg.price < params.price {
            return Err(Error::PriceTooLow);
        }

        // TODO: rounding everywhere?
        let (src_out, taker_dst_used) = {
            let taker_want_src = msg
                .price
                .src_floor_checked(taker_dst_in)
                .ok_or(Error::IntegerOverflow)?;
            // TODO: what if zero?
            if taker_want_src < self.maker_src_remaining {
                if !params.partial_fills_allowed {
                    return Err(Error::PartialFillsNotAllowed);
                }
                (taker_want_src, taker_dst_in)
            } else {
                (
                    self.maker_src_remaining,
                    msg.price
                        .dst_ceil_checked(self.maker_src_remaining)
                        .ok_or(Error::IntegerOverflow)?,
                )
            }
        };

        self.maker_src_remaining -= src_out;
        let taker_dst_refund = taker_dst_in - taker_dst_used;

        let protocol_dst_fees = params
            .protocol_fees
            .map(|p| {
                Ok::<_, Error>(ProtocolFeesCollected {
                    fee: p.fee.fee_ceil(taker_dst_used),
                    surplus: if !p.surplus.is_zero() {
                        let maker_want_dst = params
                            .price
                            .dst_ceil_checked(src_out)
                            .ok_or(Error::IntegerOverflow)?;
                        let surplus = taker_dst_used.saturating_sub(maker_want_dst);
                        p.surplus.fee_ceil(surplus)
                    } else {
                        0
                    },
                    collector: p.collector.into(),
                })
            })
            .transpose()?;

        let integrator_dst_fees: BTreeMap<Cow<AccountIdRef>, _> = params
            .integrator_fees
            .into_iter()
            .map(|(collector, fee)| (collector.into(), fee.fee_ceil(taker_dst_used)))
            .collect();

        let mut maker_dst_out = taker_dst_used;
        let mut send_fees = None;
        for (collector, fee_amount) in integrator_dst_fees
            .iter()
            .map(|(collector, amount)| (collector.as_ref(), *amount))
            // chain with protocol fees
            .chain(
                protocol_dst_fees
                    .as_ref()
                    .map(|p| {
                        p.fee
                            .checked_add(p.surplus)
                            .map(|a| (p.collector.as_ref(), a))
                            .ok_or(Error::IntegerOverflow)
                    })
                    .transpose()?,
            )
        {
            if fee_amount == 0 {
                continue;
            }
            maker_dst_out = maker_dst_out
                .checked_sub(fee_amount)
                .ok_or(Error::ExcessiveFees)?;

            send_fees = Some(send_fees.take().and_or(params.dst_token.clone().send(
                collector.to_owned(),
                fee_amount,
                Some("fee".to_string()),
                None,
                None,
                false, // no unused gas
            )));
        }

        FillEvent {
            taker: Cow::Borrowed(&sender_id),
            maker: Cow::Borrowed(&params.maker),
            src_token: Cow::Borrowed(&params.src_token),
            dst_token: Cow::Borrowed(&params.dst_token),
            taker_price: msg.price,
            maker_price: params.price,
            taker_dst_in,
            taker_dst_used,
            src_out,
            maker_dst_out,
            maker_src_remaining: self.maker_src_remaining,
            maker_receive_dst_to: params
                .receive_dst_to
                .receiver_id
                .as_deref()
                .map(Cow::Borrowed),
            taker_receive_src_to: msg.receive_src_to.receiver_id.as_deref().map(Cow::Borrowed),
            protocol_dst_fees,
            integrator_dst_fees,
        }
        .emit();

        if src_out == 0 || maker_dst_out == 0 {
            // TODO: maybe check earlier?
            return Err(Error::InsufficientAmount);
        }

        // send to maker
        let (maker_dst, maker_dst_p) = params.dst_token.send_for_resolve(
            params.receive_dst_to.receiver_id.unwrap_or(params.maker),
            maker_dst_out,
            params.receive_dst_to.memo,
            params.receive_dst_to.msg,
            params.receive_dst_to.min_gas,
            true, // unused gas
        );

        Ok(maker_dst_p
            // send to taker
            .and(
                params.src_token.send(
                    msg.receive_src_to
                        .receiver_id
                        .unwrap_or_else(|| sender_id.clone()),
                    src_out,
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
                    .return_value(maker_dst.token_type.refund_value(taker_dst_refund)?),
            )
            .into())
    }

    fn close(&mut self, signer_id: AccountId, params: Params) -> Result<Option<Promise>> {
        if !self.closed {
            let reason = if self.deadline.has_expired() {
                CloseReason::DeadlineExpired
            } else if self.maker_src_remaining == 0 && signer_id == params.maker {
                CloseReason::ByMaker
            } else if params.taker_whitelist.len() == 1
                && params.taker_whitelist.contains(&signer_id)
            {
                CloseReason::BySingleTaker
            } else {
                return Err(Error::Unauthorized);
            };

            self.close_unchecked(reason);
        }

        self.lost_found(params)
    }

    fn lost_found(&mut self, params: Params) -> Result<Option<Promise>> {
        let (sent_src, send_src_p) = self
            .closed
            .then(|| mem::take(&mut self.maker_src_remaining))
            .filter(|a| *a > 0)
            .map(|amount| {
                params.src_token.send_for_resolve(
                    params
                        .refund_src_to
                        .receiver_id
                        .unwrap_or_else(|| params.maker.clone()),
                    amount,
                    params.refund_src_to.memo,
                    params.refund_src_to.msg,
                    params.refund_src_to.min_gas,
                    true, // unused gas
                )
            })
            .unzip();

        let (sent_dst, send_dst_p) = Some(mem::take(&mut self.maker_dst_lost))
            .filter(|a| *a > 0)
            .map(|amount| {
                params.dst_token.send_for_resolve(
                    params.receive_dst_to.receiver_id.unwrap_or(params.maker),
                    amount,
                    params.receive_dst_to.memo,
                    params.receive_dst_to.msg,
                    params.receive_dst_to.min_gas,
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

        // TODO: emit event

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
