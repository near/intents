use std::str::FromStr;

use near_sdk::{AccountId, near, serde_json};

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferMessage {
    pub receiver_id: AccountId,
    pub salt: [u8; 32],
    pub msg: String,
}

impl FromStr for TransferMessage {
    type Err = serde_json::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}
