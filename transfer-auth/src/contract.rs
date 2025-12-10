use defuse_auth_call::AuthCallee;
use near_sdk::PromiseError;
use impl_tools::autoimpl;
use near_sdk::{
    AccountId, Gas, GasWeight, PanicOnDefault, Promise, PromiseOrValue, env, near,
    require,
};

use crate::TransferAuth;
use crate::event::Event;
use crate::storage::{ContractStorage, State, StateMachine};

#[near(contract_state(key = ContractStorage::STATE_KEY))]
#[derive(Debug, PanicOnDefault)]
#[autoimpl(DerefMut using self.0)]
#[autoimpl(Deref using self.0)]
pub struct Contract(ContractStorage);

#[near]
impl Contract {
    #[init]
    #[allow(clippy::missing_const_for_fn, clippy::use_self)]
    pub fn new(state_init: State) -> Self {
        Self(ContractStorage::new(state_init))
    }

    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn is_authorized_resume(&mut self,
        #[callback_result] resume_data: Result<(), PromiseError>,
    ) -> PromiseOrValue<()> {
        require!(matches!(self.0.fsm, StateMachine::Done | StateMachine::Authorized), "timeout"); //otherwise timeouted
        //NOTE: in some corener case when the promise can not be resumed(becuase of timeout) but
        //the timeout was already scheduled, the contract is in StateMachine::Authroized state so
        //we need to set it to StateMachine::Done
        self.fsm = StateMachine::Done;
        PromiseOrValue::Value(())
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
            signer_id == self.state_init.on_auth_signer,
            "Unauthorized on_auth signer"
        );

        match self.fsm {
            StateMachine::Idle => self.fsm = StateMachine::Authorized,
            StateMachine::WaitingForAuthorization(yield_id) => {
                //TODO: handle set status == false ,to authorized
                if yield_id.resume(&[]){
                    self.fsm = StateMachine::Done;
                }else{
                    self.fsm = StateMachine::Authorized;
                };
            }
            StateMachine::Authorized | StateMachine::Done=> {
                env::panic_str("already authorized");
            }
        };

        Event::Authorized.emit();
        PromiseOrValue::Value(())
    }
}

#[near]
impl TransferAuth for Contract {
    fn state(&self) -> &ContractStorage {
        &self.0
    }

    //TODO: remove
    fn cancel(&self) {
        // let storage = self
        //     .0
        //     .0
        //     .as_ref()
        //     .ok_or(Error::ContractIsBeingDestroyed)
        //     .unwrap_or_panic_display();
        //
        // require!(
        //     env::predecessor_account_id() == storage.state_init.authorizee,
        //     "Unauthorized authorizee"
        // );
        // Promise::new(env::current_account_id())
        //     .delete_account(env::signer_account_id())
        //     .detach();
    }

    fn wait_for_authorization(&mut self) -> PromiseOrValue<()> {
        if env::predecessor_account_id() != self.state_init.authorizee {
            env::panic_str("Unauthorized authorizee");
        }

        match self.fsm {
            StateMachine::Idle => {
                let (promise, yield_id) = Promise::yield_create(
                    "is_authorized_resume",
                    serde_json::json!({}).to_string(),
                    Gas::from_tgas(0),
                    GasWeight(1),
                );
                self.fsm = StateMachine::WaitingForAuthorization(yield_id);
                promise.into()
            }
            StateMachine::Authorized => {
                //TODO: address
                // guard.mark_for_cleanup();
                PromiseOrValue::Value(())
            }
            StateMachine::WaitingForAuthorization(_) => {
                env::panic_str("already waiting for authorization");
            }
            StateMachine::Done => {
                env::panic_str("already done");
            }
        }
    }
}
