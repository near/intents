use std::borrow::Cow;

use near_sdk::{
    AccountIdRef, Gas, PromiseOrValue, borsh, env::keccak256, ext_contract, json_types::U128, near,
};

/// Gas consumed by `cv_wait` in worst case (wait first, notify later).
pub const WAIT_GAS: Gas = Gas::from_tgas(7);

#[cfg(feature = "contract")]
mod contract;
mod error;
pub mod event;
pub mod storage;

pub use error::Error;
pub use storage::{ContractStorage, State, StateInit, StateMachine};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct CondVarContext<'a> {
    pub sender_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [defuse_nep245::TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    pub salt: [u8; 32],
    pub msg: Cow<'a, str>,
}

impl CondVarContext<'_> {
    pub fn hash(&self) -> [u8; 32] {
        let serialized = borsh::to_vec(&self)
            .unwrap_or_else(|_| unreachable!("CondVarContext is always serializable"));
        keccak256(&serialized)
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }
}

#[ext_contract(ext_oneshot_condvar)]
pub trait OneshotCondVar {
    fn state(&self) -> &StateMachine;
    fn view(&self) -> &State;
    fn cv_is_notified(&self) -> bool;
    fn cv_wait(&mut self) -> PromiseOrValue<bool>;
    fn cv_notify_one(&mut self);
}
