use near_sdk::{AccountId, Promise, env};

use crate::{Error, Storage};

use super::Contract;

pub struct CleanupGuard<'a>(&'a mut Contract);

impl<'a> CleanupGuard<'a> {
    pub const fn new(contract: &'a mut Contract) -> Self {
        Self(contract)
    }

    pub const fn get(&self) -> Option<&Storage> {
        self.0.0.as_ref()
    }

    pub const fn get_mut(&mut self) -> Option<&mut Storage> {
        self.0.0.as_mut()
    }

    pub fn try_get(&self) -> Result<&Storage, Error> {
        self.get().ok_or(Error::CleanupInProgress)
    }

    pub fn try_get_mut(&mut self) -> Result<&mut Storage, Error> {
        self.get_mut().ok_or(Error::CleanupInProgress)
    }

    pub fn maybe_cleanup(
        &mut self,
        beneficiary_id: impl Into<Option<AccountId>>,
    ) -> Option<Promise> {
        self.0
            .0
            .take_if(|storage| 
                // TODO: check if deadline exceeded automatically
                storage.should_cleanup()
            )
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
