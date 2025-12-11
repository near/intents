use defuse_controller::ControllerUpgradable;
use defuse_near_utils::CURRENT_ACCOUNT_ID;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{Gas, Promise, assert_one_yocto, near};

use super::{Contract, ContractExt};
use crate::Role;

const STATE_MIGRATE_DEFAULT_GAS: Gas = Gas::from_tgas(5);

#[near]
impl ControllerUpgradable for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn upgrade(
        &mut self,
        #[serializer(borsh)] code: Vec<u8>,
        #[serializer(borsh)] state_migration_gas: Option<Gas>,
    ) -> Promise {
        assert_one_yocto();

        let p = Promise::new(CURRENT_ACCOUNT_ID.clone()).deploy_contract(code);

        Self::ext_on(p)
            .with_static_gas(state_migration_gas.unwrap_or(STATE_MIGRATE_DEFAULT_GAS))
            .state_migrate()
    }

    #[private]
    fn state_migrate(&mut self) {}
}
