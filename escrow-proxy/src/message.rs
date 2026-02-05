use std::str::FromStr;

use near_sdk::{AccountId, near, serde_json};
use serde_with::{hex::Hex, serde_as};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct TransferMessage {
    pub receiver_id: AccountId,
    #[serde_as(as = "Option<Hex>")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salt: Option<[u8; 32]>,
    pub msg: String,
}

impl FromStr for TransferMessage {
    type Err = serde_json::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
use near_sdk::serde;
