use defuse_auth_call::ext_auth_callee;
use defuse_core::intents::auth::AuthCall;
use near_sdk::{AccountId, Gas, NearToken, Promise, env, near, require};

use crate::contract::{Contract, ContractExt};

#[near]
impl Contract {
    pub(crate) const DO_AUTH_CALL_MIN_GAS: Gas = Gas::from_tgas(5);

    /// Covers StateInit (NEP-616) cost when deterministic account doesn't exist yet.
    /// Only accounts for deploying via Global Contract ref (NEP-591) with <770B storage
    /// which doesn't require storage staking. If you need to attach more GAS, utilize
    /// `AuthCall::min_gas` or `NotifyOnTransfer::min_gas` and provide storage deposit separately.
    pub(crate) const STATE_INIT_GAS: Gas = Gas::from_tgas(10);

    #[private]
    pub fn do_auth_call(signer_id: AccountId, auth_call: AuthCall) -> Promise {
        if !auth_call.attached_deposit.is_zero() {
            require!(
                matches!(env::promise_result_checked(0, 0), Ok(data) if data.is_empty()),
                "near_withdraw failed",
            );
        }

        let min_gas = auth_call.min_gas();
        let mut p = Promise::new(auth_call.contract_id);

        if let Some(state_init) = auth_call.state_init {
            p = p.state_init(state_init, NearToken::ZERO);
        }

        ext_auth_callee::ext_on(p)
            .with_attached_deposit(auth_call.attached_deposit)
            .with_static_gas(min_gas)
            .on_auth(signer_id, auth_call.msg)
    }
}
