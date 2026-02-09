use std::borrow::Cow;

#[cfg(feature = "contract")]
mod contract;
mod message;
pub mod state;

use near_sdk::{AccountId, AccountIdRef, borsh, env, ext_contract, json_types::U128, near};

pub use self::message::*;
pub use state::{ContractStorage, ProxyConfig};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct CondVarContext<'a> {
    pub sender_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [defuse_nep245::TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    pub receiver_id: Cow<'a, AccountIdRef>,
    pub msg: Cow<'a, str>,
}

impl CondVarContext<'_> {
    pub fn hash(&self) -> [u8; 32] {
        let serialized = borsh::to_vec(&self).expect("CondVarContext is always serializable");
        env::keccak256(&serialized)
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }
}

#[ext_contract(ext_escrow_proxy)]
pub trait EscrowProxy {
    fn ep_config(&self) -> &ProxyConfig;
    fn ep_approve_account_id(
        &self,
        sender_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        receiver_id: AccountId,
        msg: String,
    ) -> AccountId;
}
