use near_sdk::{Gas, PromiseOrValue, ext_contract};

/// Gas consumed by `cv_wait` in worst case (wait first, notify later).
pub const WAIT_GAS: Gas = Gas::from_tgas(7);

#[cfg(feature = "contract")]
mod contract;
mod error;
pub mod event;
pub mod storage;

pub use error::Error;
pub use storage::{ContractStorage, State, StateInit, StateMachine};

#[ext_contract(ext_oneshot_condvar)]
pub trait OneshotCondVar {
    fn cv_state(&self) -> &StateMachine;
    fn cv_view(&self) -> &State;
    fn cv_is_notified(&self) -> bool;
    fn cv_wait(&mut self) -> PromiseOrValue<bool>;
    fn cv_notify_one(&mut self);
}
