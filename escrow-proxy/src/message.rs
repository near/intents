use defuse_crypto::Signature;
use defuse_token_id::nep245;
use near_sdk::{json_types::U128, near, AccountId};

pub use crate::escrow_params::{OverrideSend, Params as EscrowParams};

/// Nonce for replay protection (base64-encoded 32-byte salt)
pub type Nonce = u64;

/// Authorization message signed by relay
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FillAuthorization {
    pub escrow: AccountId,
    pub price: U128,
    pub amount: U128,
    pub token: nep245::TokenId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receive_src_to: Option<OverrideSend>,
    /// Unix timestamp in nanoseconds (epoch)
    pub deadline: U128,
    pub nonce: Nonce,
}

impl FillAuthorization {
    /// Compute SHA-256 hash over canonical JSON for signature verification
    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        let json =
            near_sdk::serde_json::to_string(self).expect("serialization should not fail");
        near_sdk::env::sha256_array(json.as_bytes())
    }
}

/// Transfer message sent via mt_transfer_call msg parameter
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct TransferMessage {
    pub authorization: FillAuthorization,
    pub escrow_params: EscrowParams,
    pub signature: Signature,
}
