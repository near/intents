use near_sdk::{AccountId, near};

pub type TokenId = String;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[near(serializers = [json, borsh])]
pub struct Token {
    pub token_id: TokenId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<AccountId>,
}
