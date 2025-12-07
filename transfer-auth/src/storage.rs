use std::{collections::BTreeMap, fmt};

use crate::error::Error;
use near_sdk::{AccountId, CryptoHash, Gas, GasWeight, Promise, YieldId, borsh, near};
use serde_with::{hex::Hex, serde_as};
use std::cell::Cell;
use thiserror::Error as ThisError;

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

// TODO: self cleanup after oneshot use
impl Drop for Fsm {
    fn drop(&mut self) {
    }
}

pub (crate) struct LazyYieldId(Cell<Option<Promise>>);

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
        let (promise, yield_id) =
            Promise::yield_create("is_authorized_resume", serde_json::json!({}).to_string(), Gas::from_tgas(0), GasWeight(1));

        self.0.set(Some(promise));
        yield_id
    }

    pub fn into_promise(self) -> Option<Promise> {
        self.0.take()
    }
}

pub (crate) enum FsmEvent<'a> {
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
        near_sdk::env::log_str(&format!("Fsm: {self:?} -> {event}"));
        let next_state = match (&self, event) {
            (Fsm::Idle, FsmEvent::Authorize) => Fsm::Authorized,
            (Fsm::Idle, FsmEvent::WaitForAuthorization(yield_id)) => {
                Fsm::AsyncVerification(yield_id.yield_create())
            },

            (Fsm::Authorized, FsmEvent::WaitForAuthorization(_)) => Fsm::Finished(true),

            (Fsm::AsyncVerification(_), FsmEvent::Timeout) => Fsm::Finished(false),
            (Fsm::AsyncVerification(yield_id), FsmEvent::Authorize) => {
                yield_id.resume(&[]);
                Fsm::AsyncVerificationSuccessful
            },

            (Fsm::AsyncVerificationSuccessful, FsmEvent::NotifyYieldedPromiseResolved) => Fsm::Finished(true),
            (Fsm::AsyncVerificationSuccessful, FsmEvent::Timeout) => Fsm::Finished(false),
            (state, event) => {
                return Err(FsmError::InvalidStateTransition);
                // near_sdk::env::panic_str(&format!("Invalid state transition state: {state:?} event: {event}"));
            }
        };
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
    pub fn authorize_multiple_times_fails(){

        let mut state_machine = Fsm::Idle;
        state_machine
            .handle(&FsmEvent::Authorize);
        state_machine
            .handle(&FsmEvent::Authorize);

        assert_eq!(state_machine, Fsm::Finished(true));
    }

    #[test]
    pub fn query_multiple_times_fails(){

        let mut state_machine = Fsm::Idle;
 
        state_machine
            .handle(&FsmEvent::WaitForAuthorization(&LazyYieldId::new())).unwrap();
        state_machine
            .handle(&FsmEvent::WaitForAuthorization(&LazyYieldId::new())).unwrap_err();

        assert_eq!(state_machine, Fsm::Finished(true));
    }


    #[test]
    pub fn authroize_before_query(){

        let mut state_machine = Fsm::Idle;

        state_machine
            .handle(&FsmEvent::Authorize).unwrap();

        state_machine
            .handle(&FsmEvent::WaitForAuthorization(&LazyYieldId::new())).unwrap_err();
    }

    #[test]
    pub fn successful_query_before_authroize(){

        let mut state_machine = Fsm::Idle;
        let yield_id = LazyYieldId::new();
        state_machine
            .handle(&FsmEvent::WaitForAuthorization(&yield_id)).unwrap();
        state_machine
            .handle(&FsmEvent::Authorize).unwrap();

        assert!(matches!(&state_machine, Fsm::AsyncVerificationSuccessful));
        state_machine.handle(&FsmEvent::NotifyYieldedPromiseResolved).unwrap();
        assert_eq!(state_machine, Fsm::Finished(true));
    }

    #[test]
    pub fn timeout_query_before_authroize(){

        let mut state_machine = Fsm::Idle;
        let yield_id = LazyYieldId::new();
        state_machine
            .handle(&FsmEvent::WaitForAuthorization(&yield_id)).unwrap();
        state_machine
            .handle(&FsmEvent::Authorize).unwrap();

        assert!(matches!(&state_machine, Fsm::AsyncVerificationSuccessful));
        state_machine.handle(&FsmEvent::Timeout).unwrap();
        assert_eq!(state_machine, Fsm::Finished(false));
    }
}

//TODO: rename
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub solver_id: AccountId,
    pub escrow_contract_id: AccountId,
    pub auth_contract: AccountId,
    pub auth_callee: AccountId,
    pub querier: AccountId,
    // #[serde_as(as = "Hex")]
    pub msg_hash: [u8; 32],
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractStorage {
    #[serde(flatten)]
    pub state_init: State,
    pub fsm: Fsm,
}

impl ContractStorage {
    pub(crate) const STATE_KEY: &[u8] = b"";

    #[inline]
    pub fn new(state: State) -> Self {
        Self {
            state_init: state,
            fsm: Fsm::Idle,
        }
    }

    pub fn init_state(state: State) -> Result<BTreeMap<Vec<u8>, Vec<u8>>, Error> {
        let state = Self::new(state);
        Ok([(
            Self::STATE_KEY.to_vec(),
            borsh::to_vec(&state).map_err(Error::Borsh)?,
        )]
        .into())
    }
}

//TODO: handle in fsm::finished
// pub struct CleanupGuard<'a>(&'a mut ContractStorage);
//
//
// impl Drop for CleanupGuard<'_> {
//     fn drop(&mut self) {
//         self.0.authorized = false;
//         self.0.yielded_promise_id = None;
//     }
// }
