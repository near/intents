use defuse_auth_call::AuthCallee;
use near_sdk::{
    env, ext_contract, near, require, AccountId, CryptoHash, Gas, PanicOnDefault, Promise,
    PromiseError, PromiseOrValue, YieldId,
};
use std::collections::HashSet;

mod message;
pub use message::AuthMessage;

#[ext_contract(ext_self)]
pub trait SelfCallbacks {
    fn is_authorized_resume(&mut self, #[callback_result] response: Result<(), PromiseError>);
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    // Authorization tracking (single authorization)
    pub solver_id: Option<AccountId>,
    pub escrow_params_hash: Option<CryptoHash>,
    pub authorized: bool,

    // Promise yielding tracking
    pub yielded_promise_id: Option<YieldId>,

    // Security: whitelist of contracts allowed to call on_auth
    pub allowed_auth_callers: HashSet<AccountId>,
}

#[near]
impl Contract {
    #[init]
    #[allow(clippy::missing_const_for_fn, clippy::use_self)]
    pub fn new(allowed_auth_callers: HashSet<AccountId>) -> Self {
        Self {
            solver_id: None,
            escrow_params_hash: None,
            authorized: false,
            yielded_promise_id: None,
            allowed_auth_callers,
        }
    }

    pub fn is_authorized(&mut self) -> PromiseOrValue<bool> {
        // Scenario A: Authorization already received (on_auth was called first)
        if self.authorized {
            env::log_str("Authorization already available");
            return PromiseOrValue::Value(true);
        }

        // Scenario B: No authorization yet - create yielded promise
        env::log_str("Creating yielded promise, waiting for on_auth");

        let (yield_id, promise) = ext_self::yield_execution()
            .with_static_gas(Gas::from_tgas(5))
            .is_authorized_resume();

        self.yielded_promise_id = Some(yield_id);

        env::log_str(&format!("Yielded promise created: {yield_id:?}"));

        PromiseOrValue::Promise(promise)
    }

    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn is_authorized_resume(
        &mut self,
        #[callback_result] response: Result<(), PromiseError>,
    ) -> bool {
        env::log_str("is_authorized_resume called");

        // Clean up state
        self.yielded_promise_id = None;

        if response == Ok(()) {
            // Promise was resumed by on_auth - return authorization status
            let result = self.authorized;
            env::log_str(&format!("Authorization result: {result}"));
            result
        } else {
            // Timeout after 200 blocks - authorization never came
            env::log_str("Authorization timeout - returning false");
            false
        }
    }
}

#[near]
impl AuthCallee for Contract {
    #[payable]
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()> {
        // Security: Validate caller is authorized
        require!(
            self.allowed_auth_callers.contains(&env::predecessor_account_id()),
            "Unauthorized caller"
        );

        env::log_str(&format!("on_auth called by {signer_id} with msg: {msg}"));

        // Parse message to extract authorization data
        let auth_msg: AuthMessage =
            serde_json::from_str(&msg).expect("Failed to parse auth message");

        // Store authorization data
        self.solver_id = Some(auth_msg.solver_id);
        self.escrow_params_hash = Some(auth_msg.escrow_params_hash);
        self.authorized = auth_msg.authorized;

        // Scenario C: If there's a yielded promise waiting, resume it
        if let Some(yield_id) = self.yielded_promise_id {
            env::log_str(&format!("Resuming yielded promise: {yield_id:?}"));
            let result = Promise::yield_resume(yield_id, Vec::<u8>::new());
            env::log_str(&format!("Resume result: {result}"));
        } else {
            env::log_str("No yielded promise to resume");
        }

        PromiseOrValue::Value(())
    }
}
