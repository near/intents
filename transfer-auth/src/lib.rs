use defuse_auth_call::AuthCallee;
use near_sdk::{
    env, near, require, serde::Serialize, AccountId, CryptoHash, Gas, GasWeight, PanicOnDefault, Promise, PromiseError, PromiseIndex, PromiseOrValue, YieldId
};
use serde_json::json;


pub enum PromiseOrPromiseIndexOrValue<T: Serialize> {
    Promise(Promise),
    PromiseIndex(PromiseIndex),
    Value(T),
}
//
// impl<T> Drop for PromiseOrPromiseIndexOrValue<T>
// where
//     T: Serialize,
// {
//     #[allow(clippy::wrong_self_convention)]
//     fn drop(& mut self) {
//         match self {
//             PromiseOrPromiseIndexOrValue::Promise(promise) => {
//                 drop(promise)
//             }
//             PromiseOrPromiseIndexOrValue::PromiseIndex(promise_index) => {
//                 env::promise_return(*promise_index);
//             }
//             PromiseOrPromiseIndexOrValue::Value(value) => {
//                 env::value_return(&serde_json::to_vec(&value).unwrap());
//             }
//         }
//     }
// }
//
// impl<T: Serialize> From<Promise> for PromiseOrPromiseIndexOrValue<T> {
//     fn from(promise: Promise) -> Self {
//         PromiseOrPromiseIndexOrValue::Promise(promise)
//     }
// }

// impl<T> PromiseOrPromiseIndexOrValue<T>
// where
//     T: Serialize,
// {
//     #[allow(clippy::wrong_self_convention)]
//     pub fn as_return(self) {
//         match self {
//             PromiseOrPromiseIndexOrValue::Promise(promise) => {
//                 promise.as_return();
//             }
//             PromiseOrPromiseIndexOrValue::PromiseIndex(promise_index) => {
//                 env::promise_return(promise_index);
//             }
//             PromiseOrPromiseIndexOrValue::Value(value) => {
//                 env::value_return(&serde_json::to_vec(&value).unwrap());
//             }
//         }
//     }
// }
//
// impl<T: Serialize> From<Promise> for PromiseOrPromiseIndexOrValue<T> {
//     fn from(promise: Promise) -> Self {
//         PromiseOrPromiseIndexOrValue::Promise(promise)
//     }
// }

mod message;
pub use message::AuthMessage;

// YieldId type alias
// type YieldId = CryptoHash;

// // Helper function to create a yielded promise
// fn promise_yield_create(
//     account_id: AccountId,
//     function_name: &str,
//     arguments: Vec<u8>,
//     gas: Gas,
//     weight: GasWeight,
// ) -> (YieldId, PromiseOrPromiseIndexOrValue<bool>) {
//     const YIELD_DATA_ID_REGISTER: u64 = 0;
//
//     // Call the low-level function
//     let yield_promise_index = env::promise_yield_create(
//         function_name,
//         &arguments,
//         gas,
//         weight,
//         YIELD_DATA_ID_REGISTER,
//     );
//
//     // Read the data_id from the register
//     let data_id_vec = env::read_register(YIELD_DATA_ID_REGISTER)
//         .expect("promise_yield_create should write data_id to register");
//
//     // Convert to CryptoHash (should always be 32 bytes)
//     let data_id: CryptoHash = data_id_vec
//         .try_into()
//         .expect("data_id should be 32 bytes");
//
//     let yield_id = data_id;
//     (yield_id, PromiseOrPromiseIndexOrValue::PromiseIndex(yield_promise_index))
// }
//
// // Helper function to resume a yielded promise
// fn promise_yield_resume(yield_id: YieldId) -> bool {
//     env::promise_yield_resume(&yield_id, &[])
// }

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
    // escrow contract id 
    pub escrow_contract_id: AccountId,
    pub auth_contract: AccountId,
    pub auth_callee: AccountId,
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
        // require!(env::predecessor_account_id() == self.state_init.querier, "Unauthorized auth contract");
        // require!(env::current_account_id() == self.state_init.auth_callee, "Unauthorized auth callee");

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


        let (promise, yield_id) = Promise::yield_create("is_authorized_resume", &serde_json::to_vec(&args).unwrap(), Gas::from_tgas(0), GasWeight(1));
        self.yielded_promise_id = Some(yield_id);
        PromiseOrValue::Promise(promise)
    }

    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn is_authorized_resume(
        &mut self,
        init_data: String, 
        #[callback_result] resume_data: Result<String, PromiseError>,
    ) -> PromiseOrValue<bool> {
        env::log_str(&format!("is_authorized_resume called with init_data: {init_data}"));
        // env::log_str(&format!("is_authorized_resume called with init_data: {init_data:?} and resume_data: {resume_data:?}"));

        // let init_data_str = String::from_utf8_lossy(&init_data);
        // env::log_str(&format!("is_authorized_resume init_data (str): {init_data_str}"));
        match resume_data {
            Ok(resume_data) => {
                env::log_str(&format!("is_authorized_resume resume_data (str): {}", resume_data));
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
        // Security: Validate caller is authorized
        require!(
            self.state_init.auth_contract != env::predecessor_account_id() ||
            self.state_init.auth_callee == signer_id,
            "on_auth_call from unauthorized predecessor"
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
