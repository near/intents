use defuse_auth_call::AuthCallee;
use near_sdk::{
    env, near, require, AccountId, CryptoHash, Gas, GasWeight, PanicOnDefault, Promise,
    PromiseOrValue, YieldId,
};

mod message;
pub use message::AuthMessage;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    // Authorization tracking (single authorization)
    state_init: TransferCallStateInit,

    pub authorized: bool,
    pub yielded_promise_id: Option<YieldId>,

}


#[near(serializers = [borsh, json])]
struct TransferCallStateInit{
    pub solver_id: AccountId,
    pub escrow_params_hash: CryptoHash,
    pub authorized_contract: AccountId,
    pub authorizer_entity: AccountId,
    pub querier: AccountId,
}
#[near]
impl Contract {
    #[init]
    #[allow(clippy::missing_const_for_fn, clippy::use_self)]
    pub fn new(state_init: TransferCallStateInit) -> Self {
        Self {
            state_init,
            authorized: false,
            yielded_promise_id: None,
        }
    }

    pub fn wait_for_authorization(&mut self) -> PromiseOrValue<bool> {
        require!(env::predecessor_account_id() == self.state_init.querier, "Unauthorized caller()");

        if self.authorized {
            return PromiseOrValue::Value(true);
        }

        if self.yielded_promise_id.is_some() {
            env::panic_str("wait_for_authorization called multiple times");
        }

        let (yield_id, promise) = Promise::yield_create(
            env::current_account_id(),
            "is_authorized_resume",
            vec![],
            Gas::from_tgas(5),
            GasWeight::default(),
        );

        self.yielded_promise_id = Some(yield_id);

        PromiseOrValue::Promise(promise)
    }

    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn is_authorized_resume(
        &mut self,
    ) -> PromiseOrValue<bool> {
        self.yielded_promise_id = None;
        PromiseOrValue::Value(self.authorized)
    }
}

#[near]
impl AuthCallee for Contract {
    #[payable]
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()> {
        // Security: Validate caller is authorized
        require!(
            self.state_init.authorized_contract != env::predecessor_account_id() ||
            self.state_init.authorizer_entity == signer_id,
            "on_auth_call from unauthorized predecessor"
        );

        env::log_str(&format!("on_auth called by {signer_id} with msg: {msg}"));

        // Parse message to extract authorization data
        let auth_msg: AuthMessage =
            serde_json::from_str(&msg).expect("Failed to parse auth message");

        if auth_msg.solver_id != self.state_init.solver_id {
            return PromiseOrValue::Value(());
        }

        if auth_msg.escrow_params_hash != self.state_init.escrow_params_hash {
            return PromiseOrValue::Value(());
        }

        if let Some(yield_id) = self.yielded_promise_id {
            let was_resumed = Promise::yield_resume(yield_id, []);
            env::log_str(&format!("Yielding promise: {:?}, status: {}", yield_id, was_resumed));
        } else {
            self.authorized = true;
        }

        PromiseOrValue::Value(())
    }
}
