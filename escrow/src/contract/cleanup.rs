use core::mem;

use near_sdk::{AccountId, Promise, env};

use crate::{Error, EscrowEvent, State, Storage};

use super::Contract;

pub struct CleanupGuard<'a>(&'a mut Contract);

impl<'a> CleanupGuard<'a> {
    pub const fn new(contract: &'a mut Contract) -> Self {
        Self(contract)
    }

    pub const fn get_mut(&mut self) -> Option<&mut Storage> {
        self.0.0.as_mut()
    }

    pub fn try_get_mut(&mut self) -> Result<&mut Storage, Error> {
        self.get_mut().ok_or(Error::CleanupInProgress)
    }

    pub fn maybe_cleanup(
        &mut self,
        beneficiary_id: impl Into<Option<AccountId>>,
    ) -> Option<Promise> {
        self.0.0.take_if(Storage::should_cleanup).map(|_storage| {
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
            && self.maker_src_remaining == 0
            && self.maker_dst_lost == 0
            && self.callbacks_in_flight == 0
    }
}
