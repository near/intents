#[cfg(feature = "auth-call")]
mod auth_call;
mod cleanup;

use near_sdk::PromiseError;
use near_sdk::{Gas, GasWeight, PanicOnDefault, Promise, PromiseOrValue, env, near, require};

use crate::TransferAuth;
use crate::event::Event;
use crate::storage::{ContractStorage, State, StateInit, StateMachine};

#[near(contract_state(key = ContractStorage::STATE_KEY))]
#[derive(Debug, PanicOnDefault)]
pub struct Contract(ContractStorage);

impl ContractStorage {
    #[inline]
    const fn as_alive(&self) -> Option<&State> {
        self.0.as_ref()
    }

    #[inline]
    fn try_as_alive(&self) -> &State {
        self.as_alive().expect("cleanup in progress")
    }
}

#[near]
impl Contract {
    #[init]
    #[allow(clippy::missing_const_for_fn, clippy::use_self)]
    pub fn new(state_init: StateInit) -> Self {
        Self(ContractStorage::init(state_init))
    }

    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn is_authorized_resume(
        &mut self,
        #[callback_result] _resume_data: Result<(), PromiseError>,
    ) -> PromiseOrValue<bool> {
        let mut guard = self.cleanup_guard();
        let state = guard.try_as_alive_mut();

        match state.state {
            StateMachine::WaitingForAuthorization(_yield_id) => {
                state.state = StateMachine::Idle;
                Event::Timeout.emit();
            }
            StateMachine::Done | StateMachine::Authorized => {
                state.state = StateMachine::Done;
            }
            StateMachine::Idle => {
                unreachable!()
            }
        }

        // NOTE: in some corner case when the promise can not be resumed (because of timeout) but
        // the timeout was already scheduled, the contract is in StateMachine::Authorized state so
        // we need to set it to StateMachine::Done
        PromiseOrValue::Value(matches!(state.state, StateMachine::Done))
    }
}

#[near]
impl Contract {
    pub(crate) fn do_authorize(&mut self) {
        let mut guard = self.cleanup_guard();
        let state = guard.try_as_alive_mut();

        match state.state {
            StateMachine::Idle => state.state = StateMachine::Authorized,
            StateMachine::WaitingForAuthorization(yield_id) => {
                if yield_id.resume(&[]).is_ok() {
                    // NOTE: Set to Authorized, not Done.
                    // is_authorized_resume callback will transition to Done.
                    // This prevents cleanup from deleting the contract before the callback runs.
                    state.state = StateMachine::Authorized;
                } else {
                    // NOTE: if resume returns Err that means that the yielded promise
                    // no longer exists although maybe it will be resumed because of timeout
                    // from the perspective of the contract it is already authorized
                    state.state = StateMachine::Authorized;
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
    fn view(&self) -> &State {
        self.0.try_as_alive()
    }

    fn state(&self) -> &StateMachine {
        &self.0.try_as_alive().state
    }

    fn is_authorized(&self) -> bool {
        self.0
            .as_alive()
            .is_some_and(|s| matches!(s.state, StateMachine::Authorized | StateMachine::Done))
    }

    fn wait_for_authorization(&mut self) -> PromiseOrValue<bool> {
        let mut guard = self.cleanup_guard();
        let state = guard.try_as_alive_mut();

        if env::predecessor_account_id() != state.state_init.authorizee {
            env::panic_str("Unauthorized authorizee");
        }

        match state.state {
            StateMachine::Idle => {
                let (promise, yield_id) = Promise::new_yield(
                    "is_authorized_resume",
                    serde_json::json!({}).to_string().as_bytes(),
                    Gas::from_tgas(0),
                    GasWeight(1),
                );
                state.state = StateMachine::WaitingForAuthorization(yield_id);
                return promise.into();
            }
            StateMachine::Authorized => {
                state.state = StateMachine::Done;
            }
            StateMachine::WaitingForAuthorization(_) => {
                env::panic_str("already waiting for authorization");
            }
            StateMachine::Done => {
                env::panic_str("already done");
            }
        }

        PromiseOrValue::Value(matches!(state.state, StateMachine::Done))
    }

    #[payable]
    fn authorize(&mut self) {
        let state = self.0.try_as_alive();
        require!(
            env::predecessor_account_id() == state.state_init.on_auth_signer,
            "Unauthorized signer"
        );

        self.do_authorize();
    }
}
