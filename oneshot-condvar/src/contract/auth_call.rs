use defuse_auth_call::AuthCallee;
use defuse_near_utils::UnwrapOrPanicError;
use near_sdk::{AccountId, PromiseOrValue, env, near, require};

use super::{Contract, ContractExt};

const ERR_MSG_NOT_EMPTY: &str = "message must be empty";
const ERR_WRONG_AUTH_CALLER: &str = "Unauthorized auth contract";
const ERR_WRONG_SIGNER: &str = "Unauthorized on_auth signer";

#[near]
impl AuthCallee for Contract {
    #[payable]
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()> {
        require!(msg.is_empty(), ERR_MSG_NOT_EMPTY);

        let state = self.0.try_as_alive().unwrap_or_panic_display();
        require!(
            env::predecessor_account_id() == state.state_init.auth_contract,
            ERR_WRONG_AUTH_CALLER
        );
        require!(signer_id == state.state_init.notifier_id, ERR_WRONG_SIGNER);

        self.do_notify();
        PromiseOrValue::Value(())
    }
}
