use defuse_admin_utils::full_access_keys::FullAccessKeys;
use near_sdk::{Promise, PublicKey, assert_one_yocto, env, near};

use super::Contract;

#[near]
impl FullAccessKeys for Contract {
    #[payable]
    fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        self.assert_owner();
        assert_one_yocto();
        Promise::new(env::current_account_id()).add_full_access_key(public_key)
    }

    #[payable]
    fn delete_key(&mut self, public_key: PublicKey) -> Promise {
        self.assert_owner();
        assert_one_yocto();
        Promise::new(env::current_account_id()).delete_key(public_key)
    }
}
