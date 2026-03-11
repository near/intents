use defuse_actions::Action;
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{AccountId, Promise, env, near};

use crate::{
    arbitrary::{ArbitraryAction, ArbitraryManager},
    contract::{Contract, ContractExt, Role},
};

#[near]
impl ArbitraryManager for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn arbitrary_call(&mut self, account_id: AccountId, action: ArbitraryAction) -> Promise {
        assert!(
            env::attached_deposit() >= action.get_deposit(),
            "Attached deposit is not enough to cover actions"
        );

        action.append(Promise::new(account_id))
    }
}
