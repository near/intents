use defuse_nep641::{OffchainMessage, Proof};
use near_account_id::AccountId;

use crate::SignatureSchema;

// TODO: docs
pub trait OffchainSignatureSchema: SignatureSchema {
    // TODO: docs
    #[must_use = "check if verification passed"]
    fn verify_offchain_msg(
        public_key: &Self::PublicKey,
        msg: &OffchainMessage,
        proof: &str,
    ) -> bool;
}

// TODO: docs
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
// #[cfg_attr(
//     feature = "borsh",
//     derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
//     cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
// )]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WalletOffchainProof {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub as_extension_id: Option<AccountId>,

    pub proof: Proof,
}
