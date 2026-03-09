use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{assert_one_yocto, near};

use crate::{
    arbitrary::{ArbitraryAction, ArbitraryManager},
    contract::{Contract, ContractExt, Role},
};

#[near]
impl ArbitraryManager for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn arbitrary_call(&mut self, action: ArbitraryAction) {
        assert_one_yocto();
    }
}
