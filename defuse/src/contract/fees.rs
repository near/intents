use super::{Contract, ContractExt, Role, intents::execute::ExecuteInspector};
use crate::fees::FeesManager;
use defuse_core::{engine::Engine, fees::Pips};
use near_plugins::{AccessControllable, Pausable, access_control_any, pause};
use near_sdk::{AccountId, assert_one_yocto, near};

#[near]
impl FeesManager for Contract {
    #[pause(name = "intents")]
    #[access_control_any(roles(Role::DAO, Role::FeesManager))]
    #[payable]
    fn set_fee(&mut self, #[allow(unused_mut)] mut fee: Pips) {
        assert_one_yocto();
        Engine::new(self, ExecuteInspector::default())
            .state_mutator()
            .set_fee(fee);
    }

    fn fee(&self) -> Pips {
        self.fees.fee
    }

    #[pause(name = "intents")]
    #[access_control_any(roles(Role::DAO, Role::FeesManager))]
    #[payable]
    fn set_fee_collector(&mut self, #[allow(unused_mut)] mut fee_collector: AccountId) {
        assert_one_yocto();
        Engine::new(self, ExecuteInspector::default())
            .state_mutator()
            .set_fee_collector(fee_collector);
    }

    fn fee_collector(&self) -> &AccountId {
        &self.fees.fee_collector
    }
}
