#[cfg(feature = "auth-call")]
mod auth_call;
mod cleanup;

use defuse_near_utils::UnwrapOrPanicError;
use near_sdk::{Gas, GasWeight, PanicOnDefault, Promise, PromiseOrValue, env, near, require};

use crate::{
    Error, OneshotCondVar,
    event::Event,
    storage::{ContractStorage, State, StateMachine},
};

const EMPTY_JSON: &[u8] = b"{}";
const ERR_UNAUTHORIZED_CALLER: &str = "Unauthorized caller";

#[near(contract_state(key = ContractStorage::STATE_KEY))]
#[derive(Debug, PanicOnDefault)]
pub struct Contract(ContractStorage);

impl ContractStorage {
    #[inline]
    const fn as_alive(&self) -> Option<&State> {
        self.0.as_ref()
    }

    #[inline]
    fn try_as_alive(&self) -> Result<&State, Error> {
        self.as_alive().ok_or(Error::CleanupInProgress)
    }
}

#[near]
impl Contract {
    #[private]
    #[allow(clippy::needless_pass_by_value)]
    pub fn cv_wait_resume(
        &mut self,
        // #[callback_result] _resume_data: Result<(), PromiseError>,
    ) -> PromiseOrValue<bool> {
        let mut guard = self.cleanup_guard();
        let state = guard.try_as_alive_mut().unwrap_or_panic_display();

        state.state = match state.state {
            // The yield promise timed out while we were waiting for a notification.
            // Reset to Idle state so the caller can retry cv_wait() if desired.
            // This is the normal timeout path when no authorization arrives in time.
            StateMachine::WaitingForNotification(_yield_id) => {
                Event::Timeout.emit();
                StateMachine::Idle
            }
            // Authorization arrived before or (in corner case) after timeout
            // in either case we want to transition from Authorized to Done
            StateMachine::Notified => StateMachine::Done,
            // This callback is only scheduled by Promise::new_yield in cv_wait(),
            // which transitions state to WaitingForNotification. We should never
            // reach this callback while in Idle state.
            StateMachine::Done | StateMachine::Idle => unreachable!(),
        };

        PromiseOrValue::Value(matches!(state.state, StateMachine::Done))
    }
}

impl Contract {
    pub(crate) fn verify_caller_and_authorize_contract(
        caller: &near_sdk::AccountId,
        state: &mut State,
    ) {
        require!(
            *caller == state.state_init.notifier_id,
            ERR_UNAUTHORIZED_CALLER
        );

        state.state = match state.state {
            StateMachine::Idle => StateMachine::Notified,
            StateMachine::WaitingForNotification(yield_id) => {
                let _ = yield_id.resume(&[]);
                // set state to authorized despite the status of yield resume.
                // in both cases we want to keep the state machine in Authorized state
                // - if yield succeeded - state will be changed to Done in callback
                // - if yield failed (timeout) - state will be changed to done on next `cv_wait`
                // call
                StateMachine::Notified
            }
            StateMachine::Done | StateMachine::Notified => {
                env::panic_str("already notified");
            }
        };

        Event::Authorized.emit();
    }
}

#[near]
impl OneshotCondVar for Contract {
    fn cv_view(&self) -> &State {
        self.0.try_as_alive().unwrap_or_panic_display()
    }

    fn cv_state(&self) -> &StateMachine {
        &self.0.try_as_alive().unwrap_or_panic_display().state
    }

    fn cv_is_notified(&self) -> bool {
        self.0
            .as_alive()
            .is_some_and(|s| matches!(s.state, StateMachine::Notified | StateMachine::Done))
    }

    fn cv_wait(&mut self) -> PromiseOrValue<bool> {
        let mut guard = self.cleanup_guard();
        let state = guard.try_as_alive_mut().unwrap_or_panic_display();

        require!(
            env::predecessor_account_id() == state.state_init.authorizee,
            "Unauthorized authorizee"
        );

        match state.state {
            StateMachine::Idle => {
                let (promise, yield_id) = Promise::new_yield(
                    "cv_wait_resume",
                    EMPTY_JSON,
                    Gas::from_tgas(0),
                    GasWeight(1),
                );
                state.state = StateMachine::WaitingForNotification(yield_id);
                return promise.into();
            }
            StateMachine::Notified => {
                state.state = StateMachine::Done;
            }
            StateMachine::WaitingForNotification(_) => {
                env::panic_str("already waiting for notification");
            }
            StateMachine::Done => {
                env::panic_str("already done");
            }
        }

        PromiseOrValue::Value(matches!(state.state, StateMachine::Done))
    }

    #[payable]
    fn cv_notify_one(&mut self) {
        let mut guard = self.cleanup_guard();
        let state = guard.try_as_alive_mut().unwrap_or_panic_display();
        Self::verify_caller_and_authorize_contract(&env::predecessor_account_id(), state);
    }
}
