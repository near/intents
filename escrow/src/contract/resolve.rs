use defuse_near_utils::UnwrapOrPanic;
use near_sdk::{Gas, near};

use crate::{Error, Result, state::State};

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
        maker_src: Option<Sent>,
        maker_dst: Option<Sent>,
    ) -> Result<()> {
        for (result_idx, (sent, lost)) in maker_src
            .map(|s| (s, &mut self.maker_src_remaining))
            .into_iter()
            .chain(maker_dst.map(|s| (s, &mut self.maker_dst_lost)))
            .enumerate()
        {
            let refund =
                sent.resolve_refund(result_idx.try_into().unwrap_or_else(|_| unreachable!()));

            // TODO: emit event if non-zero refund?
            *lost = lost.checked_add(refund).ok_or(Error::IntegerOverflow)?;
        }

        Ok(())
    }
}
