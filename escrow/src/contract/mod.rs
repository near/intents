mod cleanup;
mod resolve;
mod return_value;
mod tokens;
mod utils;

use std::{borrow::Cow, mem};

use defuse_near_utils::{MaybePromise, PromiseExt, UnwrapOrPanic};
use defuse_token_id::TokenId;

use near_sdk::{
    AccountId, PanicOnDefault, Promise, PromiseOrValue, env, near, require, serde_json,
};

use crate::{
    Action, AddSrcEvent, ContractStorage, CreateEvent, Error, Escrow, EscrowEvent,
    EscrowIntentEmit, FillAction, FillEvent, FixedParams, OpenAction, Params, Result, Storage,
    TransferMessage, contract::tokens::TokenIdTypeExt,
};

use self::{return_value::ReturnValueExt, tokens::TokenIdExt};

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
pub struct Contract(Option<ContractStorage>);

#[near]
impl Contract {
    #[init]
    pub fn escrow_init(fixed: &FixedParams, params: Params) -> Self {
        CreateEvent {
            fixed: Cow::Borrowed(&fixed),
            params: Cow::Borrowed(&params),
        }
        .emit();

        let s = ContractStorage::new(fixed, params).unwrap_or_panic();

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
    fn escrow_view(&self) -> &ContractStorage {
        self.try_as_alive().unwrap_or_panic()
    }

    // TODO: cancel_by_resolver?
    #[payable]
    fn escrow_close(&mut self, fixed_params: FixedParams) -> PromiseOrValue<bool> {
        self.close(fixed_params).unwrap_or_panic()
    }

    fn escrow_lost_found(&mut self, fixed_params: FixedParams) -> PromiseOrValue<bool> {
        self.lost_found(fixed_params).unwrap_or_panic()
    }
}

impl Contract {
    fn close(&mut self, fixed_params: FixedParams) -> Result<PromiseOrValue<bool>> {
        let mut guard = self.cleanup_guard();

        let this = guard.try_as_alive_mut()?.verify_mut(&fixed_params)?;

        Ok(if let Some(promise) = this.close(fixed_params)? {
            PromiseOrValue::Promise(promise)
        } else {
            PromiseOrValue::Value(guard.maybe_cleanup(None).is_some())
        })
    }

    fn lost_found(&mut self, fixed_params: FixedParams) -> Result<PromiseOrValue<bool>> {
        let mut guard = self.cleanup_guard();

        let this = guard.try_as_alive_mut()?.verify_mut(&fixed_params)?;

        Ok(if let Some(promise) = this.lost_found(fixed_params)? {
            PromiseOrValue::Promise(promise)
        } else {
            PromiseOrValue::Value(guard.maybe_cleanup(None).is_some())
        })
    }
}

impl Contract {
    pub fn on_receive(
        &mut self,
        sender_id: AccountId,
        token_id: TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<PromiseOrValue<u128>> {
        if amount == 0 {
            return Err(Error::InsufficientAmount);
        }

        let msg: TransferMessage = serde_json::from_str(msg)?;

        self.cleanup_guard()
            .try_as_alive_mut()?
            .verify_mut(&msg.fixed_params)?
            .on_receive(msg.fixed_params, sender_id, token_id, amount, msg.action)
    }
}

impl Storage {
    /// Returns refund amount
    fn on_receive(
        &mut self,
        fixed: FixedParams,
        // TODO: it could have been EscrowFactory who forwarded funds to us
        sender_id: AccountId,
        token_id: TokenId,
        amount: u128,
        action: Action,
    ) -> Result<PromiseOrValue<u128>> {
        // TODO: check amount non-zero
        if self.state.closed || self.params.deadline.has_expired() {
            // TODO: utilize for our needs, refund after being closed or expired?
            // TODO: what if maker wants to reopen and prolongate deadline?
            return Err(Error::Closed);
        }

        match action {
            Action::Open(open) if token_id == fixed.src_token => {
                self.on_open(fixed, sender_id, amount, open)
            }
            Action::Fill(fill) if token_id == fixed.dst_token => {
                self.on_fill(fixed, sender_id, amount, fill)
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

                        let send = fixed.dst_token.clone().send(
                            fee_collector.clone(),
                            fee_amount,
                            Some("fee".to_string()),
                            None, // TODO: msg for fee_collectors?
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

        // TODO: lost&found?
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
                    .escrow_resolve_transfers(
                        None,
                        Some(maker_dst),
                        // TODO: beneficiary_id
                        sender_id,
                    )
                    .return_value(maker_dst.token_type.refund_value(refund)?),
            )
            .into())
    }

    fn close(&mut self, fixed: FixedParams) -> Result<Option<Promise>> {
        // TODO: authority
        if !(self.state.closed
            || self.params.deadline.has_expired()
            || self.state.maker_src_remaining == 0 && fixed.maker == env::predecessor_account_id()
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

        let just_closed = !mem::replace(&mut self.state.closed, true);
        // TODO: what if already closed? maybe not allow closing twice?
        if just_closed {
            // TODO: allow retries
            // TODO: enrich event
            EscrowEvent::Close.emit();
        }

        self.lost_found(fixed)
    }

    fn lost_found(&mut self, fixed: FixedParams) -> Result<Option<Promise>> {
        let (sent_src, send_src_p) = self
            .state
            .closed
            .then(|| mem::take(&mut self.state.maker_src_remaining))
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

        // TODO: maybe don't retry dst_lost here?
        let (sent_dst, send_dst_p) = Some(mem::take(&mut self.state.maker_dst_lost))
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
                    .escrow_resolve_transfers(sent_src, sent_dst, env::predecessor_account_id()),
            )
            .into())
    }
}
