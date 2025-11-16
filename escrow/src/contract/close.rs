use near_sdk::{AccountIdRef, Promise, PromiseOrValue};

use crate::{Error, Params, Result, State, event::CloseReason};

use super::Contract;

impl Contract {
    pub(super) fn close(
        &mut self,
        signer_id: &AccountIdRef,
        params: Params,
    ) -> Result<PromiseOrValue<bool>> {
        let mut guard = self.cleanup_guard();

        let state = guard.try_as_alive_mut()?.verify_mut(&params)?;

        Ok(if let Some(promise) = state.close(signer_id, params)? {
            PromiseOrValue::Promise(promise)
        } else {
            PromiseOrValue::Value(guard.maybe_cleanup().is_some())
        })
    }
}

impl State {
    pub(super) fn close(
        &mut self,
        signer_id: &AccountIdRef,
        params: Params,
    ) -> Result<Option<Promise>> {
        if !self.closed {
            let reason = if self.deadline.has_expired() {
                CloseReason::DeadlineExpired
            } else if self.maker_src_remaining == 0 && signer_id == params.maker {
                CloseReason::ByMaker
            } else if params.taker_whitelist.len() == 1
                && params.taker_whitelist.contains(signer_id)
            {
                CloseReason::BySingleTaker
            } else {
                return Err(Error::Unauthorized);
            };

            self.close_unchecked(reason);
        }

        self.lost_found(params)
    }
}
