use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{AccountId, Promise, assert_one_yocto, near};

use crate::{
    arbitrary::{ArbitraryAction, ArbitraryManager},
    contract::{Contract, ContractExt, Role},
};

#[near]
impl ArbitraryManager for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn arbitrary_call(&mut self, contract: AccountId, action: ArbitraryAction) {
        assert_one_yocto();

        Promise::new(contract).detach();
    }
}
