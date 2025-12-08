use defuse_auth_call::AuthCallee;
use near_sdk::{
    env, ext_contract, near, require, serde::Serialize, AccountId, CryptoHash, Gas, GasWeight, PanicOnDefault, Promise, PromiseError, PromiseIndex, PromiseOrValue, YieldId
};
use serde_json::json;


use impl_tools::autoimpl;
use crate::storage::{ContractStorage, State, StateMachine};
use crate::state_machine::{LazyYieldId, StateMachineEvent};
use crate::event::Event;
use crate::TransferAuth;
use defuse_near_utils::UnwrapOrPanicError;

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
        self.fsm.handle(&StateMachineEvent::WaitForAuthorization(&mut yield_id)).unwrap_or_panic_display();

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
                self.fsm.handle(&StateMachineEvent::NotifyYieldedPromiseResolved).unwrap_or_panic_display();
                Event::Authorized.emit();
            }
            Err(err) => {
                self.fsm.handle(&StateMachineEvent::Timeout).unwrap_or_panic_display();
                Event::Timeout.emit();
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
        self.fsm.handle(&StateMachineEvent::Authorize).unwrap_or_panic_display();
        PromiseOrValue::Value(())
    }
}

#[near]
impl TransferAuth for Contract {
    fn state(&self) -> &ContractStorage {
        &self.0
    }

    fn close(&self){
        require!(env::predecessor_account_id() == self.state_init.querier || env::predecessor_account_id() == self.state_init.solver_id, "Unauthorized querier");
        Promise::new(env::current_account_id())
            .delete_account(env::signer_account_id()).detach();

    }


}
