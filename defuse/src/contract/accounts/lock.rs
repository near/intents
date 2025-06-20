use defuse_core::{accounts::AccountEvent, engine::StateView, events::DefuseEvent};
use defuse_near_utils::Lock;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{AccountId, assert_one_yocto, near};

use crate::{
    accounts::AccountForceLocker,
    contract::{Contract, ContractExt, Role},
};

#[near]
impl AccountForceLocker for Contract {
    fn is_account_locked(&self, account_id: &AccountId) -> bool {
        StateView::is_account_locked(self, account_id)
    }

    #[access_control_any(roles(Role::DAO, Role::UnrestrictedAccountLocker))]
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

    #[access_control_any(roles(Role::DAO, Role::UnrestrictedAccountUnlocker))]
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
}
