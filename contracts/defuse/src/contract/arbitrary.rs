use defuse_near_promise::{NearPromise, actions::NearAction};
use near_plugins::AccessControllable;
use near_plugins::access_control_any;
use near_sdk::{AccountId, Promise, env, near};

use super::{Contract, ContractExt, Role};
use crate::arbitrary::ArbitraryManager;

#[near]
impl ArbitraryManager for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn arbitrary_call(&mut self, receiver_id: AccountId, action: NearAction) -> Promise {
        assert!(
            matches!(
                action,
                NearAction::FunctionCall(_) | NearAction::Transfer(_)
            ),
            "Unsupported action"
        );

        let promise = NearPromise::new(receiver_id).add_action(action);

        assert!(
            env::attached_deposit() >= promise.total_deposit(),
            "Attached deposit is not enough to cover actions"
        );

        promise.build()
    }
}
