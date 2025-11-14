use core::mem;

use defuse_near_utils::UnwrapOrPanicError;
use near_sdk::{Promise, env};

use crate::{Error, Event, Result, Storage, state::State};

use super::{Contract, ContractExt};

impl Contract {
    pub(super) fn cleanup_guard(&mut self) -> CleanupGuard<'_> {
        CleanupGuard(self)
    }

    pub(super) const fn as_alive(&self) -> Option<&Storage> {
        self.0.as_ref()
    }

    pub(super) fn try_as_alive(&self) -> Result<&Storage> {
        self.as_alive().ok_or(Error::CleanupInProgress)
    }
}

pub struct CleanupGuard<'a>(&'a mut Contract);

impl<'a> CleanupGuard<'a> {
    pub const fn as_alive_mut(&mut self) -> Option<&mut Storage> {
        self.0.0.as_mut()
    }

    pub fn try_as_alive_mut(&mut self) -> Result<&mut Storage> {
        self.as_alive_mut().ok_or(Error::CleanupInProgress)
    }

    pub fn on_callback(&mut self) -> Result<&mut State> {
        let state = self
            .try_as_alive_mut()?
            // callbacks should be only executed on verified data
            .no_verify_mut();
        state.on_callback();
        Ok(state)
    }

    pub fn maybe_cleanup(&mut self) -> Option<Promise> {
        self.0
            .0
            .take_if(|s| s.no_verify_mut().should_cleanup())
            .map(|_state| {
                Event::Cleanup.emit();

                Promise::new(env::current_account_id())
                    // reimburse signer/relayer
                    .delete_account(env::signer_account_id())
            })
    }
}

impl<'a> Drop for CleanupGuard<'a> {
    fn drop(&mut self) {
        self.maybe_cleanup();
    }
}

impl State {
    pub(super) fn callback(&mut self) -> ContractExt {
        self.in_flight = self
            .in_flight
            .checked_add(1)
            .ok_or("too many callbacks in flight")
            .unwrap_or_panic_static_str();
        Contract::ext(env::current_account_id())
    }

    fn on_callback(&mut self) {
        self.in_flight = self
            .in_flight
            .checked_sub(1)
            .ok_or("unregistered callback")
            .unwrap_or_panic_static_str();
    }

    fn check_deadline_expired(&mut self) -> bool {
        let just_closed = self.deadline.has_expired() && !mem::replace(&mut self.closed, true);
        if just_closed {
            // TODO: enrich
            Event::Close.emit();
        }
        just_closed
    }

    // TODO: rename
    #[must_use]
    fn should_cleanup(&mut self) -> bool {
        // TODO: are we sure? what if we got more tokens from maker with prolonged deadline after close?
        // the answer is: we never know
        // if we allow only maker to close, then we will end up with a lot of unclosed contracts
        self.check_deadline_expired();

        self.closed
            && self.in_flight == 0
            && self.maker_src_remaining == 0
            && self.maker_dst_lost == 0
    }
}
