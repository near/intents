use std::cell::Cell;

use near_sdk::{Gas, GasWeight, Promise, YieldId, borsh, near};
use thiserror::Error as ThisError;

use crate::event::Event;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fsm {
    Idle,
    Authorized,
    AsyncVerification(YieldId),
    AsyncVerificationSuccessful,
    TimedOut,
    Finished(bool),
}

impl Drop for Fsm {
    fn drop(&mut self) {
        if matches!(self, Fsm::Finished(_)) {
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

pub enum FsmEvent<'a> {
    Authorize,
    WaitForAuthorization(&'a LazyYieldId),
    NotifyYieldedPromiseResolved,
    Timeout,
}

impl std::fmt::Display for FsmEvent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsmEvent::Authorize => write!(f, "Authorize"),
            FsmEvent::WaitForAuthorization(_) => write!(f, "WaitForAuthorization"),
            FsmEvent::NotifyYieldedPromiseResolved => write!(f, "NotifyYieldedPromiseResolved"),
            FsmEvent::Timeout => write!(f, "Timeout"),
        }
    }
}

impl Fsm {
    pub fn handle(&mut self, event: &FsmEvent<'_>) -> Result<(), FsmError> {
        let next_state = match (&self, event) {
            (Fsm::Idle, FsmEvent::Authorize) => {
                Event::Authorized.emit();
                Fsm::Authorized
            },
            (Fsm::Idle, FsmEvent::WaitForAuthorization(yield_id)) => {
                Event::AuthorizationRequested.emit();
                Fsm::AsyncVerification(yield_id.yield_create())
            }

            (Fsm::Authorized, FsmEvent::WaitForAuthorization(_)) => {
                Event::AuthorizationRequested.emit();
                Fsm::Finished(true)
            },

            (Fsm::AsyncVerification(_), FsmEvent::Timeout) => {
                Event::Timeout.emit();
                Fsm::Finished(false)
            },
            (Fsm::AsyncVerification(yield_id), FsmEvent::Authorize) => {
                Event::Authorized.emit();
                yield_id.resume(&[]);
                Fsm::AsyncVerificationSuccessful
            }

            (Fsm::AsyncVerificationSuccessful, FsmEvent::NotifyYieldedPromiseResolved) => {
                Fsm::Finished(true)
            }
            (Fsm::AsyncVerificationSuccessful, FsmEvent::Timeout) => {
                Event::Timeout.emit();
                Fsm::Finished(false)
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
        matches!(self, Fsm::Finished(true))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn authorize_multiple_times_fails() {
        let mut state_machine = Fsm::Idle;
        state_machine.handle(&FsmEvent::Authorize).unwrap();
        state_machine.handle(&FsmEvent::Authorize).unwrap_err();
    }

    #[test]
    pub fn query_multiple_times_fails() {
        let mut state_machine = Fsm::Idle;

        state_machine
            .handle(&FsmEvent::WaitForAuthorization(&LazyYieldId::new()))
            .unwrap();
        state_machine
            .handle(&FsmEvent::WaitForAuthorization(&LazyYieldId::new()))
            .unwrap_err();
    }

    #[test]
    pub fn authroize_before_query() {
        let mut state_machine = Fsm::Idle;

        state_machine.handle(&FsmEvent::Authorize).unwrap();

        state_machine
            .handle(&FsmEvent::WaitForAuthorization(&LazyYieldId::new()))
            .unwrap();
    }

    #[test]
    pub fn successful_query_before_authroize() {
        let mut state_machine = Fsm::Idle;
        let yield_id = LazyYieldId::new();
        state_machine
            .handle(&FsmEvent::WaitForAuthorization(&yield_id))
            .unwrap();
        state_machine.handle(&FsmEvent::Authorize).unwrap();

        assert!(matches!(&state_machine, Fsm::AsyncVerificationSuccessful));
        state_machine
            .handle(&FsmEvent::NotifyYieldedPromiseResolved)
            .unwrap();
        assert_eq!(state_machine, Fsm::Finished(true));
    }

    #[test]
    pub fn timeout_query_before_authroize() {
        let mut state_machine = Fsm::Idle;
        let yield_id = LazyYieldId::new();
        state_machine
            .handle(&FsmEvent::WaitForAuthorization(&yield_id))
            .unwrap();
        state_machine.handle(&FsmEvent::Authorize).unwrap();

        assert!(matches!(&state_machine, Fsm::AsyncVerificationSuccessful));
        state_machine.handle(&FsmEvent::Timeout).unwrap();
        assert_eq!(state_machine, Fsm::Finished(false));
    }
}
