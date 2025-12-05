use defuse_auth_call::AuthCallee;
use near_sdk::{
    env, ext_contract, near, require, serde::Serialize, AccountId, CryptoHash, Gas, GasWeight, PanicOnDefault, Promise, PromiseError, PromiseIndex, PromiseOrValue, YieldId
};
use serde_json::json;

mod error;
pub mod storage;

use impl_tools::autoimpl;
use storage::{ContractStorage, State};

pub enum PromiseOrPromiseIndexOrValue<T: Serialize> {
    Promise(Promise),
    PromiseIndex(PromiseIndex),
    Value(T),
}
mod message;
pub use message::AuthMessage;

#[near(contract_state(key = ContractStorage::STATE_KEY))]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[derive(Debug, PanicOnDefault)]
pub struct Contract(ContractStorage);

#[ext_contract(ext_transfer_auth)]
pub trait TransferAuth {
    fn state(&self) -> &ContractStorage;
}

#[near]
impl TransferAuth for Contract {
    fn state(&self) -> &ContractStorage {
        &self.0
    }
}


#[near]
impl Contract {
    #[init]
    #[allow(clippy::missing_const_for_fn, clippy::use_self)]
    pub fn new(state_init: State) -> Self {
        Self(ContractStorage::new(state_init))
    }

    pub fn wait_for_authorization(
        &mut self,
    ) -> PromiseOrValue<bool> {
        if env::predecessor_account_id() != self.state_init.querier {
            env::panic_str("Unauthorized querier");
        }

        if self.authorized {
            return PromiseOrValue::Value(true);
        }

        if self.yielded_promise_id.is_some() {
            env::panic_str("wait_for_authorization called multiple times 22");
        }

        // let init_value = serde_json::to_string(&b"create".to_vec()).unwrap();
        // env::log_str(&format!("wait_for_authorization called with init_value: {init_value}"));

        let init_value = serde_json::to_vec(&String::from("hello world")).unwrap();
        let args = json!({
            "init_data": "hello world"
        });
        // serde_json::to_vec(&args).unwrap();

        let (promise, yield_id) = Promise::yield_create(
            "is_authorized_resume",
            &serde_json::to_vec(&args).unwrap(),
            Gas::from_tgas(0),
            GasWeight(1),
        );
        self.0.yielded_promise_id = Some(yield_id);
        PromiseOrValue::Promise(promise)
    }

    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn is_authorized_resume(
        &mut self,
        init_data: String,
        #[callback_result] resume_data: Result<String, PromiseError>,
    ) -> PromiseOrValue<bool> {
        env::log_str(&format!(
            "is_authorized_resume called with init_data: {init_data}"
        ));
        // env::log_str(&format!("is_authorized_resume called with init_data: {init_data:?} and resume_data: {resume_data:?}"));

        // let init_data_str = String::from_utf8_lossy(&init_data);
        // env::log_str(&format!("is_authorized_resume init_data (str): {init_data_str}"));
        match resume_data {
            Ok(resume_data) => {
                env::log_str(&format!(
                    "is_authorized_resume resume_data (str): {}",
                    resume_data
                ));
            }
            Err(err) => {
                env::log_str(&format!("is_authorized_resume error (str): {err:?}"));
            }
        }
        PromiseOrValue::Value(self.authorized)
    }
}

#[near]
impl AuthCallee for Contract {
    #[payable]
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()> {
        require!(
            env::predecessor_account_id() == self.state_init.auth_contract,
            "Unauthorized auth contract"
        );
        require!(
            signer_id == self.state_init.auth_callee,
            "Unauthorized auth callee"
        );
        env::log_str(&format!("on_auth called by {signer_id} with msg: {msg}"));

        // Parse message to extract authorization data
        let auth_msg: AuthMessage =
            serde_json::from_str(&msg).expect("Failed to parse auth message");

        if auth_msg.solver_id != self.state_init.solver_id {
            return PromiseOrValue::Value(());
        }

        // if auth_msg.escrow_params_hash != self.state_init.escrow_params_hash {
        //     return PromiseOrValue::Value(());
        // }

        self.authorized = true;
        if let Some(yield_id) = self.yielded_promise_id {
            self.authorized = true;
            yield_id.resume(&serde_json::to_vec("hello world 2").unwrap()); // detached 
            // let was_resumed = promise_yield_resume(yield_id);
            // env::log_str(&format!("Yielding promise: {:?}, status: {}", yield_id, was_resumed));
        }
        PromiseOrValue::Value(())
    }
}
