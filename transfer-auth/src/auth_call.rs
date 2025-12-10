use defuse_auth_call::AuthCallee;
use near_sdk::{AccountId, PromiseOrValue, env, near, require};

use crate::contract::Contract;

#[near]
impl AuthCallee for Contract {
    #[payable]
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()> {
        let _ = msg;
        require!(
            env::predecessor_account_id() == self.state_init.auth_contract,
            "Unauthorized auth contract"
        );
        require!(
            signer_id == self.state_init.on_auth_signer,
            "Unauthorized on_auth signer"
        );

        self.authorize();
        PromiseOrValue::Value(())
    }
}
