use std::collections::{HashMap, HashSet};

use defuse_core::{
    DefuseError, Result, accounts::AccountEvent, crypto::PublicKey, engine::StateView,
    events::DefuseEvent,
};
use defuse_near_utils::Lock;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{AccountId, assert_one_yocto, near};

use crate::{
    accounts::ForceAccountManager,
    contract::{Contract, ContractExt, Role},
};

#[near]
impl ForceAccountManager for Contract {
    fn is_account_locked(&self, account_id: &AccountId) -> bool {
        StateView::is_account_locked(self, account_id)
    }

    #[access_control_any(roles(
        Role::DAO,
        Role::UnrestrictedAccountLocker,
        Role::UnrestrictedAccountManager
    ))]
    #[payable]
    fn force_lock_account(&mut self, account_id: AccountId) -> bool {
        assert_one_yocto();
        let locked = self
            .accounts
            .get_or_create(account_id.clone())
            .lock()
            .is_some();
        if locked {
            DefuseEvent::AccountLocked(AccountEvent::new(account_id, ())).emit();
        }
        locked
    }

    #[access_control_any(roles(
        Role::DAO,
        Role::UnrestrictedAccountUnlocker,
        Role::UnrestrictedAccountManager
    ))]
    #[payable]
    fn force_unlock_account(&mut self, account_id: &AccountId) -> bool {
        assert_one_yocto();
        let unlocked = self
            .accounts
            .get_mut(account_id)
            .and_then(Lock::unlock)
            .is_some();
        if unlocked {
            DefuseEvent::AccountUnlocked(AccountEvent::new(account_id, ())).emit();
        }
        unlocked
    }

    #[access_control_any(roles(
        Role::DAO,
        Role::UnrestrictedAccountLocker,
        Role::UnrestrictedAccountManager
    ))]
    #[payable]
    fn force_disable_auth_by_predecessor_ids(&mut self, account_ids: Vec<AccountId>) {
        assert_one_yocto();

        for account_id in account_ids {
            // NOTE: omit errors
            let _ = self.internal_set_auth_by_predecessor_id(&account_id, false, true);
        }
    }

    #[access_control_any(roles(
        Role::DAO,
        Role::UnrestrictedAccountUnlocker,
        Role::UnrestrictedAccountManager
    ))]
    #[payable]
    fn force_enable_auth_by_predecessor_ids(&mut self, account_ids: Vec<AccountId>) {
        assert_one_yocto();

        for account_id in account_ids {
            // NOTE: omit errors
            let _ = self.internal_set_auth_by_predecessor_id(&account_id, true, true);
        }
    }

    #[access_control_any(roles(Role::DAO, Role::UnrestrictedAccountManager))]
    #[payable]
    fn force_add_public_keys(&mut self, public_keys: HashMap<AccountId, HashSet<PublicKey>>) {
        assert_one_yocto();

        for (account_id, pks) in public_keys {
            for pk in pks {
                self.add_public_key_and_emit_event(account_id.as_ref(), pk);
            }
        }
    }

    #[access_control_any(roles(Role::DAO, Role::UnrestrictedAccountManager))]
    #[payable]
    fn force_remove_public_keys(&mut self, public_keys: HashMap<AccountId, HashSet<PublicKey>>) {
        assert_one_yocto();

        for (account_id, pks) in public_keys {
            for pk in pks {
                self.remove_public_key_and_emit_event(account_id.as_ref(), pk);
            }
        }
    }
}

impl Contract {
    pub(crate) fn internal_set_auth_by_predecessor_id(
        &mut self,
        account_id: &AccountId,
        enable: bool,
        force: bool,
    ) -> Result<bool> {
        if enable {
            let Some(account) = self.accounts.get_mut(account_id) else {
                // no need to create an account: not-yet-existing accounts
                // have auth by PREDECESSOR_ID enabled by default
                return Ok(true);
            };
            account
        } else {
            self.accounts.get_or_create(account_id.clone())
        }
        .get_mut_maybe_forced(force)
        .ok_or_else(|| DefuseError::AccountLocked(account_id.clone()))
        .map(|account| account.set_auth_by_predecessor_id(account_id, enable))
    }
}
