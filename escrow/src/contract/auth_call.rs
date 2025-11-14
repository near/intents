use defuse_auth_call::AuthCallee;
use defuse_near_utils::UnwrapOrPanic;
use near_sdk::{AccountId, PromiseOrValue, env, near, serde_json};

use crate::{
    Error, Result,
    auth_call::{Action, Message},
};

use super::{Contract, ContractExt};

#[near]
impl AuthCallee for Contract {
    // TODO: payable?
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()> {
        self.internal_on_auth(signer_id, msg).unwrap_or_panic()
    }
}

impl Contract {
    fn internal_on_auth(
        &mut self,
        signer_id: AccountId,
        msg: String,
    ) -> Result<PromiseOrValue<()>> {
        let msg: Message = serde_json::from_str(&msg)?;

        let mut guard = self.cleanup_guard();
        let this = guard.try_as_alive_mut()?.verify_mut(&msg.params)?;

        if !msg
            .params
            .auth_caller
            .as_ref()
            .is_some_and(|a| *a == env::predecessor_account_id())
        {
            return Err(Error::Unauthorized);
        }

        match msg.action {
            Action::Close(params) => Ok(this
                .close(signer_id, params)?
                .map(PromiseOrValue::Promise)
                .unwrap_or(PromiseOrValue::Value(()))),
        }
    }
}
