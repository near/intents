use defuse_auth_call::ext_auth_callee;
use defuse_core::intents::auth::AuthCall;
use defuse_near_utils::env::promise_result_checked;
use near_sdk::{AccountId, Gas, Promise, near, require};

use crate::contract::{Contract, ContractExt};

#[near]
impl Contract {
    pub(crate) const DO_AUTH_CALL_MIN_GAS: Gas = Gas::from_tgas(5);

    #[private]
    pub fn do_auth_call(signer_id: AccountId, auth_call: AuthCall) -> Promise {
        if !auth_call.attached_deposit.is_zero() {
            require!(
                matches!(promise_result_checked(0, 0), Ok(data) if data.is_empty()),
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
