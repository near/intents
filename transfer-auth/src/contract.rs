use defuse_auth_call::AuthCallee;
use near_sdk::{
    env, ext_contract, near, require, serde::Serialize, AccountId, CryptoHash, Gas, GasWeight, PanicOnDefault, Promise, PromiseError, PromiseIndex, PromiseOrValue, YieldId
};
use serde_json::json;


use impl_tools::autoimpl;
use crate::storage::{ContractStorage, State, LazyYieldId, Fsm, FsmEvent};
use crate::TransferAuth;
use crate::AuthMessage;

#[near(contract_state(key = ContractStorage::STATE_KEY))]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[derive(Debug, PanicOnDefault)]
pub struct Contract(ContractStorage);



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

        let mut yield_id = LazyYieldId::new();
        self.fsm.handle(&FsmEvent::WaitForAuthorization(&mut yield_id));

        yield_id
            .into_promise()
            .map(PromiseOrValue::Promise)
            .unwrap_or_else(|| (PromiseOrValue::Value(self.fsm.is_authorized())))
    }

    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn is_authorized_resume(
        &mut self,
        #[callback_result] resume_data: Result<(), PromiseError>,
    ) -> PromiseOrValue<bool> {
        match resume_data {
            Ok(_) => {
                env::log_str(&format!( "is_authorized_resume",));
                // self.fsm.handle(&FsmEvent::Authorize);
            }
            Err(err) => {
                env::log_str(&format!("is_authorized_resume error (str): {err:?}"));
                self.fsm.handle(&FsmEvent::Timeout);
            }
        }
        PromiseOrValue::Value(self.fsm.is_authorized())
    }
}

#[near]
impl AuthCallee for Contract {
    #[payable]
    fn on_auth(&mut self, signer_id: AccountId, msg: String) -> PromiseOrValue<()> {
        let _ = msg;
        require!(
            env::predecessor_account_id() == self.state_init.auth_contract,
            "Unauthorized auth contract"
        );
        require!(
            signer_id == self.state_init.auth_callee,
            "Unauthorized auth callee"
        );
        env::log_str(&format!("on_auth called by {signer_id} with msg: {msg}"));
        self.fsm.handle(&FsmEvent::Authorize);
        PromiseOrValue::Value(())
    }
}

#[near]
impl TransferAuth for Contract {
    fn state(&self) -> &ContractStorage {
        &self.0
    }
}
