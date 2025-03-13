use defuse_near_utils::{CURRENT_ACCOUNT_ID, UnwrapOrPanicError, method_name};
use near_plugins::{AccessControllable, Pausable, access_control_any, pause};
use near_sdk::{Allowance, Promise, PublicKey, assert_one_yocto, env, near, require};

use crate::{
    contract::{Contract, ContractExt, Role},
    intents::{Intents, RelayerKeys},
};

const EXECUTE_INTENTS_FUNC: &str = method_name!(Contract::execute_intents);

#[near]
impl RelayerKeys for Contract {
    #[pause(name = "intents")]
    #[payable]
    #[access_control_any(roles(Role::DAO, Role::RelayerKeysManager))]
    fn add_relayer_key(&mut self, public_key: PublicKey) -> Promise {
        Self::ext(CURRENT_ACCOUNT_ID.clone())
            .do_add_relayer_key(public_key.clone())
            .add_access_key_allowance(
                public_key,
                Allowance::limited(env::attached_deposit())
                    .ok_or("no deposit attached for allowance")
                    .unwrap_or_panic_static_str(),
                CURRENT_ACCOUNT_ID.clone(),
                EXECUTE_INTENTS_FUNC.into(),
            )
    }

    #[private]
    fn do_add_relayer_key(&mut self, public_key: PublicKey) {
        require!(
            self.relayer_keys.insert(public_key.clone()),
            "key already exists",
        );
    }

    #[pause(name = "intents")]
    #[access_control_any(roles(Role::DAO, Role::RelayerKeysManager))]
    #[payable]
    fn delete_relayer_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        require!(self.relayer_keys.remove(&public_key), "key not found");

        Promise::new(CURRENT_ACCOUNT_ID.clone()).delete_key(public_key)
    }
}
