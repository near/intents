use core::mem;

use defuse_near_utils::UnwrapOrPanicError;
use near_sdk::{AccountId, Promise, env};

use crate::{ContractStorage, Error, EscrowEvent, Result, State, Storage, contract::ContractExt};

use super::Contract;

impl Contract {
    pub(super) const fn cleanup_guard(&mut self) -> CleanupGuard<'_> {
        CleanupGuard::new(self)
    }

    pub(super) const fn as_alive(&self) -> Option<&ContractStorage> {
        self.0.as_ref()
    }

    pub(super) fn try_as_alive(&self) -> Result<&ContractStorage> {
        self.as_alive().ok_or(Error::CleanupInProgress)
    }
}

pub struct CleanupGuard<'a>(&'a mut Contract);

impl<'a> CleanupGuard<'a> {
    pub const fn new(contract: &'a mut Contract) -> Self {
        Self(contract)
    }

    pub const fn as_alive_mut(&mut self) -> Option<&mut ContractStorage> {
        self.0.0.as_mut()
    }

    pub fn try_as_alive_mut(&mut self) -> Result<&mut ContractStorage> {
        self.as_alive_mut().ok_or(Error::CleanupInProgress)
    }

    pub fn on_callback(&mut self) -> Result<&mut Storage> {
        let this = self
            .try_as_alive_mut()?
            // callbacks should be only executed on verified data
            .no_verify_mut();
        this.on_callback();
        Ok(this)
    }

    pub fn maybe_cleanup(
        &mut self,
        beneficiary_id: impl Into<Option<AccountId>>,
    ) -> Option<Promise> {
        self.0
            .0
            .take_if(|s| s.no_verify_mut().should_cleanup())
            .map(|_storage| {
                Promise::new(env::current_account_id()).delete_account(
                    beneficiary_id
                        .into()
                        .unwrap_or_else(env::predecessor_account_id),
                )
            })
    }
}

impl<'a> Drop for CleanupGuard<'a> {
    fn drop(&mut self) {
        self.maybe_cleanup(None);
    }
}

impl Storage {
    pub(super) fn callback(&mut self) -> ContractExt {
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
            // TODO: enrich
            EscrowEvent::Close.emit();
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

        self.state.should_cleanup()
    }
}

impl State {
    const fn should_cleanup(&self) -> bool {
        self.closed
            && self.callbacks_in_flight == 0
            && self.maker_src_remaining == 0
            && self.maker_dst_lost == 0
    }
}
