use std::borrow::Cow;

use near_sdk::{
    AccountIdRef, PromiseOrValue, borsh, env::keccak256, ext_contract, json_types::U128, near,
};

#[cfg(feature = "contract")]
mod contract;
mod error;
pub mod event;
pub mod storage;

pub use error::Error;
pub use storage::{ContractStorage, State, StateInit, StateMachine};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct TransferAuthContext<'a> {
    pub sender_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [defuse_nep245::TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    pub salt: [u8; 32],
    pub msg: Cow<'a, str>,
}

impl TransferAuthContext<'_> {
    pub fn hash(&self) -> [u8; 32] {
        let serialized = borsh::to_vec(&self)
            .unwrap_or_else(|_| unreachable!("TransferAuthContext is always serializable"));
        keccak256(&serialized)
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }
}

#[ext_contract(ext_transfer_auth)]
pub trait TransferAuth {
    fn state(&self) -> &StateMachine;
    fn view(&self) -> &State;
    fn is_authorized(&self) -> bool;
    fn wait_for_authorization(&mut self) -> PromiseOrValue<bool>;
    fn authorize(&mut self);
}
