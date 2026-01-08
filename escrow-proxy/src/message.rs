use std::str::FromStr;

use near_sdk::{AccountId, near};

pub use defuse_escrow_swap::action::{FillAction, TransferAction};
pub use defuse_escrow_swap::{OverrideSend, Params as EscrowParams};

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
