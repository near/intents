use defuse_crypto::Signature;
use defuse_token_id::TokenId;
use near_sdk::{json_types::U128, near, AccountId};

/// Destination override for token sends
#[near(serializers = [json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverrideSend {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<AccountId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_id: Option<AccountId>,
}

/// Placeholder escrow params (minimal for Phase 1)
/// TODO: Import from escrow-swap or expand as needed
#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscrowParams {
    pub maker: AccountId,
    pub src_token: TokenId,
    pub dst_token: TokenId,
}

/// Nonce for replay protection (base64-encoded 32-byte salt)
pub type Nonce = u64;

/// Authorization message signed by relay
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FillAuthorization {
    pub escrow: AccountId,
    pub price: U128,
    pub amount: U128,
    pub token: TokenId,
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
