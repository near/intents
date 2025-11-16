use std::{borrow::Cow, collections::BTreeMap};

use defuse_near_utils::{MaybePromise, PromiseExt};
use near_sdk::{AccountId, AccountIdRef, PromiseOrValue};

use crate::{
    Error, Params, Result, State,
    action::FillAction,
    contract::{
        Contract,
        return_value::ReturnValueExt,
        tokens::{Sendable, TokenIdTypeExt},
    },
    event::{EscrowIntentEmit, FillEvent, ProtocolFeesCollected},
};

impl State {
    pub(super) fn on_fill(
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
}
