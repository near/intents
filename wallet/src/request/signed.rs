use defuse_borsh_utils::adapters::{As, TimestampSeconds};
use defuse_deadline::Deadline;
use near_sdk::{AccountId, near, serde_with::base64::Base64};

use crate::Request;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedRequest {
    #[serde(flatten)]
    pub body: SignedRequestBody,

    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        schemars(with = "String")
    )]
    #[serde_as(as = "Base64")]
    pub proof: Vec<u8>,
}

// TODO: versioned?
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedRequestBody {
    /// MUST be equal to the account_id of the wallet-contract.
    pub signer_id: AccountId,

    /// MUST be equal to chain_id of the network where the wallet-contract
    /// is deployed.
    pub chain_id: String,

    pub seqno: u32,

    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<TimestampSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<TimestampSeconds<u32>>::declaration",
            ),)
        )
    )]
    #[cfg_attr(
        any(not(feature = "abi"), target_arch = "wasm32"),
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
        )
    )]
    pub valid_until: Deadline,

    // TODO: hash request or entire SignedRequestBody??? or both???
    pub request: Request,
}
