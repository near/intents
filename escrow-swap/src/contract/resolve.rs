use defuse_near_utils::UnwrapOrPanic;
use near_sdk::{Gas, near};

use crate::{
    Error, Result, State,
    event::{EscrowIntentEmit, Event, MakerSent},
};

use super::{Contract, ContractExt, tokens::Sent};

#[near]
impl Contract {
    pub(super) const ESCROW_RESOLVE_TRANSFERS_GAS: Gas = Gas::from_tgas(10);

    #[private]
    pub fn escrow_resolve_transfers(
        &mut self,
        maker_src: Option<Sent>,
        maker_dst: Option<Sent>,
    ) -> bool {
        self.resolve_transfers(maker_src, maker_dst)
            .unwrap_or_panic()
    }
}

impl Contract {
    fn resolve_transfers(
        &mut self,
        maker_src: Option<Sent>,
        maker_dst: Option<Sent>,
    ) -> Result<bool> {
        let mut guard = self.cleanup_guard();

        guard
            .on_callback()?
            .resolve_transfers(maker_src, maker_dst)?;

        Ok(guard.maybe_cleanup().is_some())
    }
}

impl State {
    fn resolve_transfers(
        &mut self,
        mut maker_src: Option<Sent>,
        mut maker_dst: Option<Sent>,
    ) -> Result<()> {
        for (result_idx, (sent, lost)) in maker_src
            .as_mut()
            .map(|s| (s, &mut self.maker_src_remaining))
            .into_iter()
            .chain(maker_dst.as_mut().map(|s| (s, &mut self.maker_dst_lost)))
            .enumerate()
        {
            let refund =
                sent.resolve_refund(result_idx.try_into().unwrap_or_else(|_| unreachable!()));

            *lost = lost.checked_add(refund).ok_or(Error::IntegerOverflow)?;
            sent.amount = refund;
        }

        let lost = MakerSent::from_sent(maker_src, maker_dst);
        if !lost.is_empty() {
            Event::MakerLost(lost).emit();
        }

        Ok(())
    }
}

impl MakerSent {
    #[inline]
    pub(super) fn from_sent(src: Option<Sent>, dst: Option<Sent>) -> Self {
        Self {
            src: src.map_or(0, |s| s.amount),
            dst: dst.map_or(0, |s| s.amount),
        }
    }
}
