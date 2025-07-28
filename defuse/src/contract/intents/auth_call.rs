use defuse_auth_call::ext_auth_callee;
use defuse_core::intents::auth::AuthCall;
use near_sdk::{AccountId, Gas, Promise, PromiseResult, env, near, require};

use crate::contract::{Contract, ContractExt};

#[near]
impl Contract {
    pub(crate) const DO_AUTH_CALL_MIN_GAS: Gas = Gas::from_tgas(5);

    #[must_use]
    #[private]
    pub fn do_auth_call(signer_id: AccountId, auth_call: AuthCall) -> Promise {
        if !auth_call.attached_deposit.is_zero() {
            require!(
                matches!(env::promise_result(0), PromiseResult::Successful(data) if data.is_empty()),
                "near_withdraw failed",
            );
        }

        let min_gas = auth_call.min_gas();

        ext_auth_callee::ext(auth_call.contract_id)
            .with_attached_deposit(auth_call.attached_deposit)
            .with_static_gas(min_gas)
            .on_auth(signer_id, auth_call.msg)
    }
}
