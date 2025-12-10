use near_sdk::{YieldId, near};
use thiserror::Error as ThisError;

// enum State2 {
//     Idle,
//     WaitingForAuthorization(YieldId), 
//     Authorized(),
// }

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum StateMachine {
    Idle,
    WaitingForAuthorization(YieldId),
    Authorized,
    Done,
}

// impl Drop for StateMachine {
//     fn drop(&mut self) {
//         //TODO: drop when contract is state is none
//         // if matches!(self, StateMachine::Finished(_)) {
//         //     Event::Destroy.emit();
//         //     Promise::new(near_sdk::env::current_account_id()).delete_account(near_sdk::env::signer_account_id()).detach()
//         // }
//     }
// }

// pub struct LazyYieldId(Cell<Option<Promise>>);

#[derive(Debug, ThisError)]
pub enum FsmError {
    #[error("InvalidStateTransition")]
    InvalidStateTransition,
}

// impl LazyYieldId {
//     pub fn new() -> Self {
//         //TODO: onecall
//         Self(Cell::new(None))
//     }
//
//     pub fn yield_create(&self) -> YieldId {
//         let (promise, yield_id) = Promise::yield_create(
//             "is_authorized_resume",
//             serde_json::json!({}).to_string(),
//             Gas::from_tgas(0),
//             GasWeight(1),
//         );
//
//         self.0.set(Some(promise));
//         yield_id
//     }
//
//     pub fn into_promise(self) -> Option<Promise> {
//         self.0.take()
//     }
// }

// pub enum StateMachineEvent<'a> {
//     Authorize,
//     WaitForAuthorization(&'a LazyYieldId),
//     NotifyYieldedPromiseResolved,
//     Timeout,
// }

// impl std::fmt::Display for StateMachineEvent<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             StateMachineEvent::Authorize => write!(f, "Authorize"),
//             StateMachineEvent::WaitForAuthorization(_) => write!(f, "WaitForAuthorization"),
//             StateMachineEvent::NotifyYieldedPromiseResolved => write!(f, "NotifyYieldedPromiseResolved"),
//             StateMachineEvent::Timeout => write!(f, "Timeout"),
//         }
//     }
// }

// impl StateMachine {
//     pub fn handle(&mut self, event: &StateMachineEvent<'_>) -> Result<(), FsmError> {
//         let next_state = match (&self, event) {
//             (StateMachine::Idle, StateMachineEvent::Authorize) => {
//                 Event::Authorized.emit();
//                 StateMachine::Authorized
//             },
//             (StateMachine::Idle, StateMachineEvent::WaitForAuthorization(yield_id)) => {
//                 Event::AuthorizationRequested.emit();
//                 StateMachine::AsyncVerification(yield_id.yield_create())
//             }
//
//             (StateMachine::Authorized, StateMachineEvent::WaitForAuthorization(_)) => {
//                 Event::AuthorizationRequested.emit();
//                 StateMachine::Finished(true)
//             },
//
//             (StateMachine::AsyncVerification(_), StateMachineEvent::Timeout) => {
//                 Event::Timeout.emit();
//                 StateMachine::Finished(false)
//             },
//             (StateMachine::AsyncVerification(yield_id), StateMachineEvent::Authorize) => {
//                 Event::Authorized.emit();
//                 yield_id.resume(&[]);
//                 StateMachine::AsyncVerificationSuccessful
//             }
//
//             (StateMachine::AsyncVerificationSuccessful, StateMachineEvent::NotifyYieldedPromiseResolved) => {
//                 StateMachine::Finished(true)
//             }
//             (StateMachine::AsyncVerificationSuccessful, StateMachineEvent::Timeout) => {
//                 Event::Timeout.emit();
//                 StateMachine::Finished(false)
//             },
//             (state, event) => {
//                 return Err(FsmError::InvalidStateTransition);
//                 // near_sdk::env::panic_str(&format!("Invalid state transition state: {state:?} event: {event}"));
//             }
//         };
//         near_sdk::env::log_str(&format!("{self:?} -----{event}----> {next_state:?}"));
//         *self = next_state;
//         Ok(())
//     }
//
//     pub fn is_authorized(&self) -> bool {
//         matches!(self, StateMachine::Finished(true))
//     }
// }

