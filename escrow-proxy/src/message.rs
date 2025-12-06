use std::str::FromStr;

use defuse_crypto::Signature;
use defuse_deadline::Deadline;
use defuse_token_id::nep245;
use near_sdk::{AccountId, json_types::U128, near};
use serde_with::{hex::Hex, serde_as};

pub use crate::escrow_params::{OverrideSend, Params as EscrowParams};

/// Nonce for replay protection (base64-encoded 32-byte salt)
pub type Nonce = u64;

//TODO: use actual struct from escrow-swap
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FillAction {
    pub price: u64,
    pub deadline: Deadline,
}

/// Transfer message sent via mt_transfer_call msg parameter
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferMessage {
    pub fill_action: FillAction,

    pub escrow_params: EscrowParams,

    // #[serde_as(as = "Hex")]
    pub salt: [u8; 32],
}

impl FromStr for TransferMessage {
    type Err = serde_json::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).map_err(Into::into)
    }
}
