use defuse_auth_call::ext_auth_callee;
use defuse_core::intents::auth::AuthCall;
use near_sdk::{AccountId, Gas, Promise, PromiseResult, env, near, require};

use crate::contract::{Contract, ContractExt};

#[near]
impl Contract {
    pub(crate) const DO_AUTH_CALL_MIN_GAS: Gas = Gas::from_tgas(5);
    // TODO: exact value
    pub(crate) const STATE_INIT_GAS: Gas = Gas::from_tgas(10);

    #[private]
    pub fn do_auth_call(signer_id: AccountId, auth_call: AuthCall) -> Promise {
        if !auth_call.is_zero_total_amount() {
            require!(
                matches!(env::promise_result(0), PromiseResult::Successful(data) if data.is_empty()),
                "near_withdraw failed",
            );
        }

        let min_gas = auth_call.min_gas();
        let mut p = Promise::new(auth_call.contract_id);

        if let Some(state_init) = auth_call.state_init {
            p = p.state_init(state_init.state_init, state_init.amount);
        }

        ext_auth_callee::ext_on(p)
            .with_attached_deposit(auth_call.attached_deposit)
            .with_static_gas(min_gas)
            .on_auth(signer_id, auth_call.msg)
    }
}
