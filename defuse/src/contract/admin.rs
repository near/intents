use defuse_admin_utils::full_access_keys::FullAccessKeys;
use defuse_near_utils::CURRENT_ACCOUNT_ID;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{AccountId, Promise, PublicKey, assert_one_yocto, near};

use crate::accounts::AccountForceLocker;

use super::{Contract, ContractExt, Role};

#[near]
impl FullAccessKeys for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(CURRENT_ACCOUNT_ID.clone()).add_full_access_key(public_key)
    }

    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn delete_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(CURRENT_ACCOUNT_ID.clone()).delete_key(public_key)
    }
}

#[near]
impl AccountForceLocker for Contract {
    #[access_control_any(roles(Role::DAO, Role::UnrestrictedAccountLocker))]
    #[payable]
    fn force_lock_account(&mut self, account_id: AccountId) {
        assert_one_yocto();
        todo!()
    }

    #[access_control_any(roles(Role::DAO, Role::UnrestrictedAccountUnlocker))]
    #[payable]
    fn force_unlock_account(&mut self, account_id: AccountId) {
        assert_one_yocto();
        todo!()
    }
}
