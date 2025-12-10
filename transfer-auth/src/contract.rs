use impl_tools::autoimpl;
use near_sdk::PromiseError;
use near_sdk::{
    Gas, GasWeight, PanicOnDefault, Promise, PromiseOrValue, env, near, require,
};

use crate::TransferAuth;
use crate::event::Event;
use crate::storage::{ContractStorage, StateInit, StateMachine};

#[near(contract_state(key = ContractStorage::STATE_KEY))]
#[derive(Debug, PanicOnDefault)]
#[autoimpl(DerefMut using self.0)]
#[autoimpl(Deref using self.0)]
pub struct Contract(ContractStorage);

#[near]
impl Contract {
    #[init]
    #[allow(clippy::missing_const_for_fn, clippy::use_self)]
    pub fn new(state_init: StateInit) -> Self {
        Self(ContractStorage::new(state_init))
    }

    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn is_authorized_resume(
        &mut self,
        #[callback_result] resume_data: Result<(), PromiseError>,
    ) -> PromiseOrValue<bool> {
        match self.state {
            StateMachine::WaitingForAuthorization(yield_id) => {
                self.state = StateMachine::Idle;
                Event::Timeout.emit();
            }
            StateMachine::Done | StateMachine::Authorized => {
                self.state = StateMachine::Done;
            }
            StateMachine::Idle => {
                unreachable!()
            }
        }

        //NOTE: in some corener case when the promise can not be resumed(becuase of timeout) but
        //the timeout was already scheduled, the contract is in StateMachine::Authroized state so
        //we need to set it to StateMachine::Done
        PromiseOrValue::Value(matches!(self.state, StateMachine::Done))
    }
}

#[near]
impl Contract {
    pub fn authorized(&mut self) {
        require!(
            env::predecessor_account_id() == self.state_init.on_auth_signer,
            "Unauthorized signer"
        );

        self.authorize();
    }

    fn authorize(&mut self) {
        match self.state {
            StateMachine::Idle => self.state = StateMachine::Authorized,
            StateMachine::WaitingForAuthorization(yield_id) => {
                if yield_id.resume(&[]) {
                    self.state = StateMachine::Done;
                } else {
                    //NOTE: if resume returns false that means that it the yielded promise
                    //no longer exists although maybe it will be resumed because of timeout
                    //from the perspective of the contract it is already authorized
                    self.state = StateMachine::Authorized;
                };
            }
            StateMachine::Authorized | StateMachine::Done => {
                env::panic_str("already authorized");
            }
        };

        Event::Authorized.emit();
    }
}

#[near]
impl TransferAuth for Contract {
    fn view(&self) -> &ContractStorage {
        &self.0
    }

    fn state(&self) -> &StateMachine {
        &self.state
    }

    fn is_authorized(&self) -> bool {
        matches!(self.state, StateMachine::Authorized| StateMachine::Done)
    }

    fn wait_for_authorization(&mut self) -> PromiseOrValue<bool> {
        if env::predecessor_account_id() != self.state_init.authorizee {
            env::panic_str("Unauthorized authorizee");
        }

        match self.state {
            StateMachine::Idle => {
                let (promise, yield_id) = Promise::yield_create(
                    "is_authorized_resume",
                    serde_json::json!({}).to_string(),
                    Gas::from_tgas(0),
                    GasWeight(1),
                );
                self.state = StateMachine::WaitingForAuthorization(yield_id);
                return promise.into()
            }
            StateMachine::Authorized => {
                self.state = StateMachine::Done;
            }
            StateMachine::WaitingForAuthorization(_) => {
                env::panic_str("already waiting for authorization");
            }
            StateMachine::Done => {
                env::panic_str("already done");
            }
        }

        PromiseOrValue::Value(matches!(self.state, StateMachine::Done))
    }
}
