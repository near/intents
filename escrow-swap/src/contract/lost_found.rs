use core::mem;

use near_sdk::{Promise, PromiseOrValue};

use crate::{
    Result,
    contract::{Contract, tokens::Sendable},
    event::{EscrowIntentEmit, Event, MakerSent},
    state::{Params, State},
};

impl Contract {
    pub(super) fn lost_found(&mut self, params: Params) -> Result<PromiseOrValue<bool>> {
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
    pub(super) fn lost_found(&mut self, params: Params) -> Result<Option<Promise>> {
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

        Event::MakerRefunded(MakerSent::from_sent(sent_src, sent_dst)).emit();

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
