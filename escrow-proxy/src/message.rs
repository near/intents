use std::str::FromStr;

use near_sdk::json_types::U128;
use near_sdk::{near, AccountId};

pub use defuse_escrow_swap::{OverrideSend, Params as EscrowParams};
pub use defuse_escrow_swap::action::{FillAction, TransferAction};

/// Nonce for replay protection (base64-encoded 32-byte salt)
pub type Nonce = u64;

/// Transfer message sent via mt_transfer_call msg parameter to the proxy.
///
/// This message is a superset of escrow-swap's `TransferMessage` with an additional
/// `salt` field for transfer-auth derivation. The field names match escrow-swap's
/// format so the message can be forwarded directly.
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferMessage {
    pub receiver_id: AccountId,
    pub salt: [u8; 32],
}

impl FromStr for TransferMessage {
    type Err = serde_json::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).map_err(Into::into)
    }
}
