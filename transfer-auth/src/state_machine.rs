use std::cell::Cell;

use near_sdk::{Gas, GasWeight, Promise, YieldId, borsh, near};
use thiserror::Error as ThisError;

use crate::event::Event;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMachine {
    Idle,
    Authorized,
    AsyncVerification(YieldId),
    AsyncVerificationSuccessful,
    TimedOut,
    Finished(bool),
}

impl Drop for StateMachine {
    fn drop(&mut self) {
        if matches!(self, StateMachine::Finished(_)) {
            Event::Destroy.emit();
            Promise::new(near_sdk::env::current_account_id()).delete_account(near_sdk::env::signer_account_id()).detach()
        }
    }
}

pub struct LazyYieldId(Cell<Option<Promise>>);

#[derive(Debug, ThisError)]
pub enum FsmError {
    #[error("InvalidStateTransition")]
    InvalidStateTransition,
}

impl LazyYieldId {
    pub fn new() -> Self {
        Self(Cell::new(None))
    }

    pub fn yield_create(&self) -> YieldId {
        let (promise, yield_id) = Promise::yield_create(
            "is_authorized_resume",
            serde_json::json!({}).to_string(),
            Gas::from_tgas(0),
            GasWeight(1),
        );

        self.0.set(Some(promise));
        yield_id
    }

    pub fn into_promise(self) -> Option<Promise> {
        self.0.take()
    }
}

pub enum StateMachineEvent<'a> {
    Authorize,
    WaitForAuthorization(&'a LazyYieldId),
    NotifyYieldedPromiseResolved,
    Timeout,
}

impl std::fmt::Display for StateMachineEvent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateMachineEvent::Authorize => write!(f, "Authorize"),
            StateMachineEvent::WaitForAuthorization(_) => write!(f, "WaitForAuthorization"),
            StateMachineEvent::NotifyYieldedPromiseResolved => write!(f, "NotifyYieldedPromiseResolved"),
            StateMachineEvent::Timeout => write!(f, "Timeout"),
        }
    }
}

impl StateMachine {
    pub fn handle(&mut self, event: &StateMachineEvent<'_>) -> Result<(), FsmError> {
        let next_state = match (&self, event) {
            (StateMachine::Idle, StateMachineEvent::Authorize) => {
                Event::Authorized.emit();
                StateMachine::Authorized
            },
            (StateMachine::Idle, StateMachineEvent::WaitForAuthorization(yield_id)) => {
                Event::AuthorizationRequested.emit();
                StateMachine::AsyncVerification(yield_id.yield_create())
            }

            (StateMachine::Authorized, StateMachineEvent::WaitForAuthorization(_)) => {
                Event::AuthorizationRequested.emit();
                StateMachine::Finished(true)
            },

            (StateMachine::AsyncVerification(_), StateMachineEvent::Timeout) => {
                Event::Timeout.emit();
                StateMachine::Finished(false)
            },
            (StateMachine::AsyncVerification(yield_id), StateMachineEvent::Authorize) => {
                Event::Authorized.emit();
                yield_id.resume(&[]);
                StateMachine::AsyncVerificationSuccessful
            }

            (StateMachine::AsyncVerificationSuccessful, StateMachineEvent::NotifyYieldedPromiseResolved) => {
                StateMachine::Finished(true)
            }
            (StateMachine::AsyncVerificationSuccessful, StateMachineEvent::Timeout) => {
                Event::Timeout.emit();
                StateMachine::Finished(false)
            },
            (state, event) => {
                return Err(FsmError::InvalidStateTransition);
                // near_sdk::env::panic_str(&format!("Invalid state transition state: {state:?} event: {event}"));
            }
        };
        near_sdk::env::log_str(&format!("{self:?} -----{event}----> {next_state:?}"));
        *self = next_state;
        Ok(())
    }

    pub fn is_authorized(&self) -> bool {
        matches!(self, StateMachine::Finished(true))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn authorize_multiple_times_fails() {
        let mut state_machine = StateMachine::Idle;
        state_machine.handle(&StateMachineEvent::Authorize).unwrap();
        state_machine.handle(&StateMachineEvent::Authorize).unwrap_err();
    }

    #[test]
    pub fn query_multiple_times_fails() {
        let mut state_machine = StateMachine::Idle;

        state_machine
            .handle(&StateMachineEvent::WaitForAuthorization(&LazyYieldId::new()))
            .unwrap();
        state_machine
            .handle(&StateMachineEvent::WaitForAuthorization(&LazyYieldId::new()))
            .unwrap_err();
    }

    #[test]
    pub fn authroize_before_query() {
        let mut state_machine = StateMachine::Idle;

        state_machine.handle(&StateMachineEvent::Authorize).unwrap();

        state_machine
            .handle(&StateMachineEvent::WaitForAuthorization(&LazyYieldId::new()))
            .unwrap();
    }

    #[test]
    pub fn successful_query_before_authroize() {
        let mut state_machine = StateMachine::Idle;
        let yield_id = LazyYieldId::new();
        state_machine
            .handle(&StateMachineEvent::WaitForAuthorization(&yield_id))
            .unwrap();
        state_machine.handle(&StateMachineEvent::Authorize).unwrap();

        assert!(matches!(&state_machine, StateMachine::AsyncVerificationSuccessful));
        state_machine
            .handle(&StateMachineEvent::NotifyYieldedPromiseResolved)
            .unwrap();
        assert_eq!(state_machine, StateMachine::Finished(true));
    }

    #[test]
    pub fn timeout_query_before_authroize() {
        let mut state_machine = StateMachine::Idle;
        let yield_id = LazyYieldId::new();
        state_machine
            .handle(&StateMachineEvent::WaitForAuthorization(&yield_id))
            .unwrap();
        state_machine.handle(&StateMachineEvent::Authorize).unwrap();

        assert!(matches!(&state_machine, StateMachine::AsyncVerificationSuccessful));
        state_machine.handle(&StateMachineEvent::Timeout).unwrap();
        assert_eq!(state_machine, StateMachine::Finished(false));
    }
}