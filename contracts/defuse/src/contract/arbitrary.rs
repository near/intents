use defuse_near_promise::{NearPromise, actions::NearAction};
use near_plugins::AccessControllable;
use near_plugins::access_control_any;
use near_sdk::require;
use near_sdk::{AccountId, Promise, near};

use super::{Contract, ContractExt, Role};
use crate::arbitrary::ArbitraryManager;

#[near]
impl ArbitraryManager for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn arbitrary_call(&mut self, receiver_id: AccountId, action: NearAction) -> Promise {
        require!(
            matches!(
                action,
                NearAction::FunctionCall(_) | NearAction::Transfer(_)
            ),
            "Unsupported action"
        );

        // NOTE: Given that it is allowed tto spend contract balance by arbitrary call,
        // the refund in case of failure should also go to the intents contract
        NearPromise::new(receiver_id).add_action(action).build()
    }
}
