use std::borrow::Cow;

use crate::storage::{ContractStorage, State, StateMachine};
use near_sdk::{borsh, env::keccak256, ext_contract, json_types::U128, near, AccountIdRef, PromiseOrValue};

#[cfg(feature = "contract")]
mod contract;
mod error;
pub mod event;
pub mod storage;


#[cfg(feature = "test-utils")]
pub mod ext;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct TransferAuthContext<'a> {
    pub sender_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [defuse_nep245::TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    pub msg: Cow<'a, str>,
}

impl TransferAuthContext<'_> {
    pub fn hash(&self) -> [u8; 32] {
        let serialized = borsh::to_vec(&self)
          .unwrap_or_else(|_| unreachable!("TransferAuthContext is always serializable"));
        keccak256(&serialized).try_into()
          .unwrap_or_else(|_| unreachable!())
    }
}

#[ext_contract(ext_transfer_auth)]
pub trait TransferAuth {
    fn state(&self) -> &StateMachine;
    fn view(&self) -> &ContractStorage;
    fn is_authorized(&self) -> bool;
    fn wait_for_authorization(&mut self) -> PromiseOrValue<bool>;
}
