mod cleanup;
mod state;
mod tokens;
mod utils;

use std::{borrow::Cow, mem};

use defuse_near_utils::{UnwrapOrPanic, UnwrapOrPanicError};
use defuse_token_id::{TokenId, TokenIdType};

use near_sdk::{
    AccountId, Gas, GasWeight, NearToken, PanicOnDefault, Promise, PromiseOrValue, PromiseResult,
    env,
    json_types::U128,
    near, require,
    serde_json::{self},
};
use serde_with::{DisplayFromStr, serde_as};
use strum::IntoDiscriminant;

use crate::{
    Action, AddSrcEvent, CreateEvent, Error, Escrow, EscrowEvent, EscrowIntentEmit, FillAction,
    FillEvent, FixedParams, OpenAction, Params, Result, Storage, TransferMessage,
};

use self::{
    cleanup::CleanupGuard,
    tokens::Sendable,
    utils::{MaybePromise, PromiseExt},
};

// mod old;

// TODO: lost&found?

// TODO: solver: create_subescrow and lock NEAR on it

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
// TODO: recovery() method with 0 src_remaining or without msg

// TODO: keep number of pending promises
#[near(contract_state)] // TODO: (key = "")
#[derive(Debug, PanicOnDefault)]
pub struct Contract(Option<Storage>);

#[near]
impl Escrow for Contract {
    fn view(&self) -> &Storage {
        self.try_as_alive().unwrap_or_panic()
    }

    // TODO: cancel_by_resolver?
    #[payable]
    fn close(&mut self, fixed_params: FixedParams) -> PromiseOrValue<bool> {
        self.cleanup_guard()
            .internal_close(fixed_params)
            .unwrap_or_panic()
    }

    fn lost_found(&mut self, fixed_params: FixedParams) -> PromiseOrValue<bool> {
        self.cleanup_guard()
            .internal_lost_found(fixed_params)
            .unwrap_or_panic()
    }

    // TODO: separate lost_found method to be able to claim before deadline
    // expired
}

#[near]
impl Contract {
    const CLOSE_GAS: Gas = Gas::from_tgas(10);
    const RESOLVE_LOST_FOUND_GAS: Gas = Gas::from_tgas(10);

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

        Self(Some(s))
    }

    #[private]
    pub fn resolve_lost_found(
        &mut self,
        maker_src: Option<SentAsset>,
        maker_dst: Option<SentAsset>,
        beneficiary_id: AccountId,
    ) -> bool {
        self.cleanup_guard()
            .internal_resolve_lost_found(maker_src, maker_dst, beneficiary_id)
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
        mut self,
        sender_id: AccountId,
        token_id: TokenId,
        amount: u128,
        msg: &str,
    ) -> Result<PromiseOrValue<u128>> {
        self.try_get_mut()?
            .on_receive(sender_id, token_id, amount, msg)
    }

    fn internal_close(&mut self, fixed: FixedParams) -> Result<PromiseOrValue<bool>> {
        let this = self.try_get_mut()?;
        this.verify(&fixed)?;

        // TODO: authority
        if !(this.params.deadline.has_expired()
            || this.state.maker_src_remaining == 0 && env::predecessor_account_id() == fixed.maker
            || fixed.taker_whitelist == [env::predecessor_account_id()].into())
        {
            // TODO: allow to close permissionlessly if src_remaining == 0
            // TODO: what if more assets are about to arrive? allow only for maker
            // TODO: allow force close by authority (`force: bool` param?)
            // TODO: different error

            // TODO:
            // return Err(Error::DeadlineNotExpired);
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
        self.internal_lost_found(fixed)
    }

    fn internal_lost_found(&mut self, fixed: FixedParams) -> Result<PromiseOrValue<bool>> {
        let this = self.try_get_mut()?;
        this.verify(&fixed)?;

        // if this.state.callbacks_in_flight > 0 {
        //     let (promise_idx, yield_id) = promise_yield_create(
        //         // TODO: retry or lost_found()?
        //         // TODO: beneficiary predecessor_id
        //         "close",
        //         &serde_json::to_vec(&json!({
        //             "fixed_params": fixed,
        //         }))
        //         .unwrap_or_else(|_| unreachable!()),
        //         Self::CLOSE_GAS,
        //         GasWeight(1),
        //     );

        //     // TODO: what if replaces the old one?
        //     // TODO: maybe not store at all? it will be resumed afterwards
        //     debug_assert_eq!(this.state.yield_id.replace(yield_id), None);
        //     // TODO: return
        //     return todo!();
        // }

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
                    .with_static_gas(Contract::RESOLVE_LOST_FOUND_GAS)
                    .with_unused_gas_weight(0)
                    .resolve_lost_found(sent_src, sent_dst, env::predecessor_account_id()),
            )
            .into())
    }

    fn internal_resolve_lost_found(
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
            let refund = sent.resolve(result_idx.try_into().unwrap_or_else(|_| unreachable!()));

            // TODO: emit event if non-zero refund?
            *lost = lost.checked_add(refund).ok_or(Error::IntegerOverflow)?;
        }

        Ok(self.maybe_cleanup(beneficiary_id).is_some())
    }

    // #[must_use]
    // fn do_cleanup(&mut self, beneficiary_id: AccountId) -> Promise {
    //     // TODO: can we make it all in Drop?
    //     // emit event
    //     *self = Self::Cleanup;
    //     // NOTE: Unfortunately, we can't refund `storage_deposit`s on src and dst
    //     // tokens back to maker or any other beneficiary, since
    //     // `storage_unregister()` internally detaches transfer Promise, so we
    //     // don't know when it arrives and can't schedule the cleanup afterwards.
    //     Promise::new(env::current_account_id()).delete_account(beneficiary_id)
    // }
}

impl Storage {
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

        let refund_value = match &maker_dst.asset_type {
            #[cfg(feature = "nep141")]
            TokenIdType::Nep141 => serde_json::to_vec(&U128(refund)),
            #[cfg(feature = "nep245")]
            TokenIdType::Nep245 => serde_json::to_vec(&[U128(refund)]),
        }?;

        let resolve = self
            .callback()
            .with_static_gas(Contract::RESOLVE_LOST_FOUND_GAS)
            .with_unused_gas_weight(0)
            .resolve_lost_found(
                None,
                Some(maker_dst),
                // TODO: beneficiary_id
                sender_id.clone(),
            )
            .just_return(refund_value);

        // TODO: lost&found?
        Ok(maker_dst_p
            // send to taker
            .and(fixed.src_asset.send(
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

        // TODO: resume lost_found?
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

// #[must_use]
// #[derive(Debug)]
// struct WithCleanup<T> {
//     value: T,
//     cleanup_to: Option<AccountId>,
// }

// impl<T> WithCleanup<T> {
//     pub fn new(value: T, cleanup_to: impl Into<Option<AccountId>>) -> Self {
//         Self {
//             value,
//             cleanup_to: cleanup_to.into(),
//         }
//     }

//     pub fn no_cleanup(value: T) -> Self {
//         Self::new(value, None)
//     }

//     pub fn into_inner(self) -> (T, Option<AccountId>) {
//         (self.value, self.cleanup_to)
//     }
// }

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
pub struct SentAsset {
    asset_type: TokenIdType,

    #[serde_as(as = "DisplayFromStr")]
    amount: u128,

    #[serde(default, skip_serializing_if = "::core::ops::Not::not")]
    is_call: bool,
}

impl SentAsset {
    // TODO
    /// Returns refund
    pub fn resolve(&self, result_idx: u64) -> u128 {
        let used = match env::promise_result(result_idx) {
            PromiseResult::Successful(value) => {
                if self.is_call {
                    match self.asset_type {
                        #[cfg(feature = "nep141")]
                        TokenIdType::Nep141 => {
                            // `ft_transfer_call` returns successfully transferred amount
                            serde_json::from_slice::<U128>(&value).unwrap_or_default().0
                        }
                        #[cfg(feature = "nep245")]
                        TokenIdType::Nep245 => {
                            // `ft_transfer_call` returns successfully transferred amount
                            serde_json::from_slice::<[U128; 1]>(&value).unwrap_or_default()[0].0
                        }
                    }
                    .min(self.amount)
                } else if value.is_empty() {
                    // `ft_transfer` returns empty result on success
                    self.amount
                } else {
                    0
                }
            }
            PromiseResult::Failed => {
                if self.is_call {
                    // do not refund on failed `ft_transfer_call` due to
                    // NEP-141 vulnerability: `ft_resolve_transfer` fails to
                    // read result of `ft_on_transfer` due to insufficient gas
                    self.amount
                } else {
                    0
                }
            }
        };

        self.amount.saturating_sub(used)
    }
}

const JUST_RETURN_GAS: Gas = Gas::from_tgas(5); // TODO: 3?

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn just_return() {
    if let Some(input) = env::input() {
        env::value_return(&input);
    }
}

trait JustReturn: Sized {
    fn just_return(self, value: Vec<u8>) -> Self;
}

impl JustReturn for Promise {
    fn just_return(self, value: Vec<u8>) -> Self {
        self.function_call_weight(
            "just_return".to_string(),
            value,
            NearToken::from_yoctonear(0),
            JUST_RETURN_GAS,
            GasWeight(0),
        )
    }
}
