use defuse_near_promise::{NearPromise, actions::NearAction};
use near_plugins::{AccessControllable, access_control_any};
use near_sdk::{env, near, require};

use super::{Contract, ContractExt, Role};
use crate::admin_call::AdminCallManager;

#[near]
impl AdminCallManager for Contract {
    #[access_control_any(roles(Role::DAO))]
    #[payable]
    fn admin_call(&mut self, promises: Vec<NearPromise>) {
        require!(
            !env::attached_deposit().is_zero(),
            "requires attached deposit of at least 1 yoctoNEAR"
        );

        for p in promises {
            require!(
                p.actions.iter().all(|action| {
                    matches!(
                        action,
                        NearAction::FunctionCall(_) | NearAction::Transfer(_)
                    )
                }),
                "unsupported action"
            );

            // NOTE: Given that it is allowed to spend contract balance by arbitrary call,
            // refund goes to the intents contract in case of failure
            p.build().detach();
        }
    }
}
