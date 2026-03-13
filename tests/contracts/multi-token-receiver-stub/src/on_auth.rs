use defuse_auth_call::AuthCallee;
use near_sdk::{AccountId, PromiseOrValue, near};

use crate::{Contract, ContractExt};

#[near]
impl AuthCallee for Contract {
    #[payable]
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()> {
        let _ = signer_id;
        let _ = msg;
        PromiseOrValue::Value(())
    }
}
