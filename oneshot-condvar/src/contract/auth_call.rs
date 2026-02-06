use defuse_auth_call::AuthCallee;
use defuse_near_utils::UnwrapOrPanicError;
use near_sdk::{AccountId, PromiseOrValue, env, near, require};

use super::{Contract, ContractExt};

const ERR_MSG_NOT_EMPTY: &str = "message must be empty";
const ERR_WRONG_AUTH_CALLER: &str = "unauthorized on_auth_caller";

#[near]
impl AuthCallee for Contract {
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()> {
        require!(msg.is_empty(), ERR_MSG_NOT_EMPTY);

        let mut guard = self.cleanup_guard();
        let state = guard.try_as_alive_mut().unwrap_or_panic_display();
        require!(
            env::predecessor_account_id() == state.config.on_auth_caller,
            ERR_WRONG_AUTH_CALLER
        );
        Self::verify_caller_and_authorize_contract(&signer_id, state);
        PromiseOrValue::Value(())
    }
}
