use near_sdk::{Promise, env};

use crate::{ContractStorage, Error, State, Status, event::Event};

use super::Contract;

impl Contract {
    #[inline]
    pub(super) fn cleanup_guard(&mut self) -> CleanupGuard<'_> {
        CleanupGuard(&mut self.0)
    }
}

pub struct CleanupGuard<'a>(&'a mut ContractStorage);

impl<'a> CleanupGuard<'a> {
    #[inline]
    pub const fn as_alive(&self) -> Option<&State> {
        self.0.0.as_ref()
    }

    #[inline]
    pub const fn as_alive_mut(&mut self) -> Option<&mut State> {
        self.0.0.as_mut()
    }

    #[inline]
    pub fn try_as_alive_mut(&mut self) -> Result<&mut State, Error> {
        self.as_alive_mut().ok_or(Error::CleanupInProgress)
    }

    #[must_use]
    pub fn maybe_cleanup(&mut self) -> Option<Promise> {
        self.0
            .0
            .take_if(|s| matches!(s.state, Status::Done))
            .map(|_state| {
                Event::Cleanup.emit();
                Promise::new(env::current_account_id()).delete_account(env::signer_account_id())
            })
    }
}

impl Drop for CleanupGuard<'_> {
    fn drop(&mut self) {
        self.maybe_cleanup().map(Promise::detach);
    }
}
