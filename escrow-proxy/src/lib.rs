use std::borrow::Cow;

#[cfg(feature = "contract")]
mod contract;
mod message;
pub mod state;

use near_sdk::{
    AccountId, AccountIdRef, CryptoHash, GlobalContractId, borsh, env, ext_contract,
    json_types::U128, near,
};

pub use message::*;
pub use state::ProxyConfig;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct CondVarContext<'a> {
    pub escrow_contract_id: Cow<'a, GlobalContractId>,
    pub sender_id: Cow<'a, AccountIdRef>,
    pub token_ids: Cow<'a, [defuse_nep245::TokenId]>,
    pub amounts: Cow<'a, [U128]>,
    pub salt: [u8; 32],
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
    fn config(&self) -> &ProxyConfig;
    fn context_hash(&self, context: CondVarContext<'static>) -> CryptoHash;
    fn oneshot_address(
        &self,
        taker_id: AccountId,
        token_ids: Vec<defuse_nep245::TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> AccountId;
}
