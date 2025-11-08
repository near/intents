mod state;
mod tokens;
mod utils;

use std::{borrow::Cow, mem};

use defuse_near_utils::{UnwrapOrPanic, UnwrapOrPanicError, time::now};
use defuse_token_id::TokenId;
use impl_tools::autoimpl;
use near_sdk::{
    AccountId, Gas, PanicOnDefault, Promise, PromiseOrValue, env, json_types::U128, near, require,
    serde_json,
};

#[cfg(feature = "nep141")]
use crate::contract::tokens::Token;
use crate::{
    Action, AddSrcEvent, ContractState, CreateEvent, Error, Escrow, EscrowEvent, EscrowIntentEmit,
    FillAction, FillEvent, FixedParams, OpenAction, Params, Result, Storage, TransferMessage,
};

use self::utils::{MaybePromise, PromiseExt};

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
#[near(contract_state)] // TODO: (key = "")
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[derive(Debug, PanicOnDefault)]
pub struct Contract(ContractState);

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

        Self(ContractState::Alive(s))
    }
}

#[near]
impl Escrow for Contract {
    fn view(&self) -> &ContractState {
        &self.0
    }

    // TODO: cancel_by_resolver?
    #[payable]
    fn close(&mut self, fixed_params: FixedParams) -> PromiseOrValue<U128> {
        self.close(fixed_params).unwrap_or_panic()
    }
}

#[near]
impl Contract {
    const RESOLVE_TRANSFERS_GAS: Gas = Gas::from_tgas(10);

    const RESOLVE_MAKER_GAS: Gas = Gas::from_tgas(10);

    #[private]
    pub fn resolve_transfers(
        &mut self,
        beneficiary_id: AccountId,
        return_value: serde_json::Value,
    ) -> serde_json::Value {
        let (result, cleanup) = self
            .try_as_alive()
            .unwrap_or_panic()
            .resolve_transfers(return_value)
            .into_inner();

        if cleanup {
            // detached
            let _ = self.do_cleanup(beneficiary_id);
        }

        result
    }

    // #[private]
    // // TODO: was it dst or src (i.e. close)?
    // pub fn resolve_maker(
    //     &mut self,
    //     amount: U128,
    //     is_call: bool,
    //     beneficiary_id: AccountId,
    // ) -> U128 {
    //     let used = resolve_mt_transfer(0, amount.0, is_call);

    //     let refund = amount.0.saturating_sub(used);
    //     if refund > 0 {
    //         self.state.maker_src_remaining = self
    //             .state
    //             .maker_src_remaining
    //             .checked_add(refund)
    //             // TODO: is it?
    //             .unwrap_or_else(|| unreachable!());
    //         // TODO: emit event
    //     }

    //     // detach promise
    //     let _ = self.maybe_cleanup(beneficiary_id);

    //     U128(refund)
    //     // TODO: maybe delete self?
    // }
}

impl Contract {
    fn on_receive(
        &mut self,
        sender_id: AccountId,
        token_id: TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<PromiseOrValue<u128>> {
        self.try_as_alive()?
            .on_receive(sender_id, token_id, amount, msg)
    }

    #[must_use]
    fn do_cleanup(&mut self, beneficiary_id: AccountId) -> Promise {
        self.0 = ContractState::Cleanup;
        // NOTE: Unfortunately, we can't refund `storage_deposit`s on src and dst
        // tokens back to maker or any other beneficiary, since
        // `storage_unregister()` internally detaches transfer Promise, so we
        // don't know when it arrives and can't schedule the cleanup afterwards.
        Promise::new(env::current_account_id()).delete_account(beneficiary_id)
    }

    fn close(&mut self, fixed_params: FixedParams) -> Result<PromiseOrValue<U128>> {
        let (result, cleanup) = self.try_as_alive()?.close(fixed_params)?.into_inner();
        if cleanup {
            // detached
            let _ = self.do_cleanup(todo!());
        }

        Ok(result)
    }

    const fn as_alive(&mut self) -> Option<&mut Storage> {
        match &mut self.0 {
            ContractState::Alive(storage) => Some(storage),
            ContractState::Cleanup => None,
        }
    }

    fn try_as_alive(&mut self) -> Result<&mut Storage> {
        self.as_alive().ok_or(Error::CleanupInProgress)
    }
}

impl Storage {
    fn close(&mut self, fixed_params: FixedParams) -> Result<WithCleanup<PromiseOrValue<U128>>> {
        // TODO: verify fixed params
        // TODO
        if !self.params.deadline.has_expired() {
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
        // TODO: do not emit event twice
        EscrowEvent::Close.emit();

        if self.should_cleanup() {
            return Ok(WithCleanup::new(PromiseOrValue::Value(U128(0)), true));
        }

        let refund_to = fixed_params
            .refund_src_to
            .receiver_id
            .unwrap_or(fixed_params.maker);

        let refund = mem::take(&mut self.state.maker_src_remaining);

        let is_call = fixed_params.refund_src_to.msg.is_some();
        Ok(WithCleanup::no_cleanup(
            Self::send(
                fixed_params.src_asset,
                refund_to.clone(),
                refund,
                fixed_params.refund_src_to.memo,
                fixed_params.refund_src_to.msg,
                fixed_params.refund_src_to.min_gas,
                true,
            )
            .then(
                self.callback()
                    .with_static_gas(Contract::RESOLVE_TRANSFERS_GAS)
                    .with_unused_gas_weight(0)
                    // TODO
                    .resolve_transfers(
                        refund_to,
                        serde_json::to_value(&U128(refund)).unwrap_or_else(|_| unreachable!()),
                    ),
            )
            // .then(
            //     Self::ext(env::current_account_id())
            //         .with_static_gas(Self::RESOLVE_MAKER_GAS)
            //         .with_unused_gas_weight(0)
            //         .resolve_maker(U128(refund), is_call, refund_to),
            // )
            .into(),
        ))
    }

    /// Returns refund amount
    fn on_receive(
        &mut self,
        sender_id: AccountId,
        token_id: TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<PromiseOrValue<u128>> {
        // TODO: check amount non-zero
        if self.state.closed || self.params.deadline.has_expired() {
            // TODO: utilize for our needs, refund after being closed or expired?
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

                        sends = Some(sends.take().and_or(Self::send(
                            fixed.dst_asset.clone(),
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

        let resolve = self
            .callback()
            .with_static_gas(Contract::RESOLVE_TRANSFERS_GAS)
            .with_unused_gas_weight(0)
            .resolve_transfers(
                fixed
                    .refund_src_to
                    .receiver_id
                    .unwrap_or_else(|| fixed.maker.clone()),
                match &fixed.dst_asset {
                    TokenId::Nep141(_) => serde_json::to_value(&U128(refund)),
                    TokenId::Nep245(_) => serde_json::to_value(&[U128(refund)]),
                }?,
            );

        // TODO: lost&found?
        // send to maker
        Ok(Self::send(
            fixed.dst_asset,
            fixed.receive_dst_to.receiver_id.unwrap_or(fixed.maker),
            maker_dst_amount, // TODO: check non-zero
            fixed.receive_dst_to.memo,
            fixed.receive_dst_to.msg,
            fixed.receive_dst_to.min_gas,
            true, // unused gas
        )
        // send to taker
        .and(Self::send(
            fixed.src_asset,
            msg.receive_src_to.receiver_id.unwrap_or(sender_id),
            taker_src_amount,
            msg.receive_src_to.memo,
            msg.receive_src_to.msg,
            msg.receive_src_to.min_gas,
            true, // unused gas
        ))
        .maybe_and(send_fees)
        .then(resolve)
        .into())
    }

    fn send(
        token: TokenId,
        receiver_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
        min_gas: Option<Gas>,
        unused_gas: bool,
    ) -> Promise {
        match token {
            #[cfg(feature = "nep141")]
            TokenId::Nep141(token) => {
                token.send(receiver_id, amount, memo, msg, min_gas, unused_gas)
            }
            #[cfg(feature = "nep245")]
            TokenId::Nep245(token) => {
                token.send(receiver_id, amount, memo, msg, min_gas, unused_gas)
            }
        }
    }

    fn resolve_transfers(
        &mut self,
        return_value: serde_json::Value,
    ) -> WithCleanup<serde_json::Value> {
        self.on_callback();

        // TODO

        WithCleanup::new(return_value, self.should_cleanup())
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

    fn check_deadline_expired(&mut self) -> bool {
        let just_closed =
            self.params.deadline.has_expired() && !mem::replace(&mut self.state.closed, true);
        if just_closed {
            EscrowEvent::Close.emit();
        }
        just_closed
    }

    // TODO: rename
    #[must_use]
    fn should_cleanup(&mut self) -> bool {
        // TODO: are we sure? what if we got more tokens from maker with prolonged deadline after close?
        self.check_deadline_expired();

        self.state.should_cleanup()
    }
}

#[derive(Debug)]
pub struct WithCleanup<T> {
    value: T,
    cleanup: bool,
}

impl<T> WithCleanup<T> {
    pub const fn new(value: T, cleanup: bool) -> Self {
        Self { value, cleanup }
    }

    pub const fn with_cleanup(value: T) -> Self {
        Self::new(value, true)
    }

    pub const fn no_cleanup(value: T) -> Self {
        Self::new(value, false)
    }

    pub fn into_inner(self) -> (T, bool) {
        (self.value, self.cleanup)
    }
}
