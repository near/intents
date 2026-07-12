use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::AccountId;
use serde::{Deserialize, Serialize};

pub type TokenId = String;

#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema, ::borsh::BorshSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Token {
    pub token_id: TokenId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<AccountId>,
}
