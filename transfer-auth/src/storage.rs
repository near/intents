use std::{collections::BTreeMap, fmt};

use crate::error::Error;
use near_sdk::{AccountId, CryptoHash, Gas, GasWeight, Promise, YieldId, borsh, near};
use serde_with::{hex::Hex, serde_as};
use std::cell::Cell;

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

pub (crate) struct LazyYieldId(Cell<Option<Promise>>);


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
    pub fn handle(&mut self, event: &FsmEvent<'_>) {
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
                near_sdk::env::panic_str(&format!("Invalid state transition state: {state:?} event: {event}"));
            }
        };
        *self = next_state;
    }

    pub fn is_authorized(&self) -> bool {
        matches!(self, Fsm::Finished(true))
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
