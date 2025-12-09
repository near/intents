use defuse_admin_utils::full_access_keys::FullAccessKeys;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{Promise, PublicKey, assert_one_yocto, env, near};

use super::{Contract, ContractExt, Role};

#[near]
impl FullAccessKeys for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(env::current_account_id()).add_full_access_key(public_key)
    }

    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn delete_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(env::current_account_id()).delete_key(public_key)
    }
}
