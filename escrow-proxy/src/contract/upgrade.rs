use defuse_controller::ControllerUpgradable;
use near_sdk::{Gas, Promise, assert_one_yocto, env, near};

#[cfg(feature = "abi")]
use super::ContractExt;
use super::Contract;

const STATE_MIGRATE_DEFAULT_GAS: Gas = Gas::from_tgas(5);

#[near]
impl ControllerUpgradable for Contract {
    #[payable]
    fn upgrade(
        &mut self,
        #[serializer(borsh)] code: Vec<u8>,
        #[serializer(borsh)] state_migration_gas: Option<Gas>,
    ) -> Promise {
        self.assert_owner();
        assert_one_yocto();

        let p = Promise::new(env::current_account_id()).deploy_contract(code);

        Self::ext_on(p)
            .with_static_gas(state_migration_gas.unwrap_or(STATE_MIGRATE_DEFAULT_GAS))
            .state_migrate()
    }

    #[private]
    fn state_migrate(&mut self) {}
}
