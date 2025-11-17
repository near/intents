use std::{borrow::Cow, collections::BTreeMap};

use defuse_near_utils::{PromiseExt, UnwrapOrPanic};
#[cfg(feature = "nep141")]
use near_sdk::{AccountId, AccountIdRef, Promise, PromiseOrValue};

use crate::{
    Error, Params, ProtocolFees, Result, State,
    action::FillAction,
    event::{EscrowIntentEmit, FillEvent, ProtocolFeesCollected},
    price::Price,
    token_id::TokenId,
};

use super::{Contract, return_value::ReturnValueExt, tokens::Sendable};

impl State {
    pub(super) fn fill(
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

        let (taker_src_out, taker_dst_used) =
            self.taker_swap(taker_dst_in, msg.price, params.partial_fills_allowed)?;

        if taker_src_out == 0 {
            return Err(Error::InsufficientAmount);
        }

        self.maker_src_remaining -= taker_src_out;

        let protocol_dst_fees = params
            .protocol_fees
            .map(|p| p.collect(taker_src_out, taker_dst_used, params.price))
            .transpose()?;

        let integrator_dst_fees: BTreeMap<Cow<AccountIdRef>, _> = params
            .integrator_fees
            .into_iter()
            .map(|(collector, fee)| (collector.into(), fee.fee_ceil(taker_dst_used)))
            .collect();

        let mut maker_dst_out = taker_dst_used;
        let send_fees = Self::collect_fees(
            &params.dst_token,
            protocol_dst_fees.as_ref(),
            &integrator_dst_fees,
            &mut maker_dst_out,
        )?;

        FillEvent {
            taker: Cow::Borrowed(&sender_id),
            maker: Cow::Borrowed(&params.maker),
            src_token: Cow::Borrowed(&params.src_token),
            dst_token: Cow::Borrowed(&params.dst_token),
            taker_price: msg.price,
            maker_price: params.price,
            taker_dst_in,
            taker_dst_used,
            taker_src_out,
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

        if maker_dst_out == 0 {
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

        // send to taker (no resolve)
        let taker_src_p = params.src_token.send(
            msg.receive_src_to
                .receiver_id
                .unwrap_or_else(|| sender_id.clone()),
            taker_src_out,
            msg.receive_src_to.memo,
            msg.receive_src_to.msg,
            msg.receive_src_to.min_gas,
            true, // unused gas
        );

        Ok(maker_dst_p
            .and(taker_src_p)
            .and_maybe(send_fees)
            .then(
                self.callback()
                    .with_static_gas(Contract::ESCROW_RESOLVE_TRANSFERS_GAS)
                    .with_unused_gas_weight(0)
                    .escrow_resolve_transfers(None, Some(maker_dst))
                    .return_value(maker_dst.refund_value(taker_dst_in - taker_dst_used)?),
            )
            .into())
    }

    fn taker_swap(
        &self,
        taker_dst_in: u128,
        taker_price: Price,
        partial_fills_allowed: bool,
    ) -> Result<(u128, u128)> {
        // TODO: rounding everywhere?
        let taker_want_src = taker_price
            .src_floor_checked(taker_dst_in)
            .ok_or(Error::IntegerOverflow)?;
        // TODO: what if zero?
        if taker_want_src < self.maker_src_remaining {
            if !partial_fills_allowed {
                return Err(Error::PartialFillsNotAllowed);
            }
            Ok((taker_want_src, taker_dst_in))
        } else {
            Ok((
                self.maker_src_remaining,
                taker_price
                    .dst_ceil_checked(self.maker_src_remaining)
                    .ok_or(Error::IntegerOverflow)?,
            ))
        }
    }

    fn collect_fees(
        token: &TokenId,
        protocol_fees: Option<&ProtocolFeesCollected>,
        integrator_fees: &BTreeMap<Cow<AccountIdRef>, u128>,
        out: &mut u128,
    ) -> Result<Option<Promise>> {
        Ok(protocol_fees
            .map(ProtocolFeesCollected::to_collector_amount)
            .transpose()?
            .into_iter()
            .chain(
                integrator_fees
                    .iter()
                    .map(|(collector, amount)| (collector.as_ref(), *amount)),
            )
            .filter(|(_, fee_amount)| *fee_amount > 0)
            .inspect(|(_, fee_amount)| {
                *out = out
                    .checked_sub(*fee_amount)
                    .ok_or(Error::ExcessiveFees)
                    .unwrap_or_panic(); // avoid too much nesting
            })
            .map(|(collector, fee_amount)| {
                token.clone().send(
                    collector.to_owned(),
                    fee_amount,
                    Some("fee".to_string()),
                    None,
                    None,
                    false, // no unused gas
                )
            })
            .reduce(Promise::and))
    }
}

impl ProtocolFees {
    fn collect(
        self,
        src_out: u128,
        taker_dst_used: u128,
        maker_price: Price,
    ) -> Result<ProtocolFeesCollected<'static>> {
        Ok(ProtocolFeesCollected {
            fee: self.fee.fee_ceil(taker_dst_used),
            surplus: if !self.surplus.is_zero() {
                let maker_want_dst = maker_price
                    .dst_ceil_checked(src_out)
                    .ok_or(Error::IntegerOverflow)?;
                let surplus = taker_dst_used.saturating_sub(maker_want_dst);
                self.surplus.fee_ceil(surplus)
            } else {
                0
            },
            collector: self.collector.into(),
        })
    }
}

impl<'a> ProtocolFeesCollected<'a> {
    fn to_collector_amount(&'a self) -> Result<(&'a AccountIdRef, u128)> {
        self.total()
            .map(|a| (self.collector.as_ref(), a))
            .ok_or(Error::ExcessiveFees)
    }
}
