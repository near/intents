use std::borrow::Cow;

use crate::storage::{ContractStorage, State, StateMachine};
use near_sdk::{env::keccak256, ext_contract, json_types::U128, near, AccountIdRef, PromiseOrValue};

#[cfg(feature = "contract")]
mod contract;
mod error;
pub mod event;
pub mod state_machine;
pub mod storage;


#[cfg(feature = "test-utils")]
pub mod ext;

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferAuthContext<'a> {
    pub sender_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [defuse_nep245::TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    pub msg: Cow<'a, str>,
}

impl TransferAuthContext<'_> {
    pub fn hash(&self) -> [u8; 32] {
        let serialized = serde_json::to_string(&self).unwrap();
        keccak256(serialized.as_bytes()).try_into().unwrap()
    }
}

#[ext_contract(ext_transfer_auth)]
pub trait TransferAuth {
    fn state(&self) -> &StateMachine;
    fn view(&self) -> &ContractStorage;
    fn is_authorized(&self) -> bool;
    //TODO: change to void
    fn wait_for_authorization(&mut self) -> PromiseOrValue<bool>;
}
