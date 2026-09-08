use defuse_near_promise::{NearPromise, actions::NearAction};
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{AccountId, Promise, env, near, require};

use super::{Contract, ContractExt, Role};
use crate::arbitrary_call::ArbitraryManager;

#[near]
impl ArbitraryManager for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn arbitrary_call(&mut self, receiver_id: AccountId, action: NearAction) -> Promise {
        require!(
            !env::attached_deposit().is_zero(),
            "requires attached deposit of at least 1 yoctoNEAR"
        );

        require!(
            matches!(
                action,
                NearAction::FunctionCall(_) | NearAction::Transfer(_)
            ),
            "Unsupported action"
        );

        // NOTE: Given that it is allowed to spend contract balance by arbitrary call,
        // the refund in case of failure should also go to the intents contract
        NearPromise::new(receiver_id).add_action(action).build()
    }
}
