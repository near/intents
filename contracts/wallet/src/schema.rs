use defuse_nep641::{OffchainMessage, Proof};
use near_account_id::AccountId;

use crate::{ChainId, RequestMessage};

/// Signature schema used by [`Wallet`](crate::contract::Wallet) contract
/// variant.
///
/// By design, each wallet contract variant implements its own schema and
/// gets deployed separately.
pub trait SignatureSchema {
    /// Public key used by this schema and [stored](field@crate::State::public_key)
    /// in the contract's state.
    ///
    ///
    /// Its [`Display`](core::fmt::Display) implementation is returned from
    /// [`w_public_key()`](crate::contract::Wallet::w_public_key) contract
    /// method.
    type PublicKey;

    /// Verify given proof over the request message in respect to the
    /// public key and return whether verification passed.
    ///
    /// Used by the `w_execute_signed(msg, proof)` contract method.
    #[must_use = "check if verification passed"]
    fn verify_request_msg(public_key: &Self::PublicKey, msg: &RequestMessage, proof: &str) -> bool;

    // TODO: docs, naming
    #[must_use = "check if verification passed"]
    fn verify_offchain_msg(
        public_key: &Self::PublicKey,
        msg: &OffchainMessage,
        proof: &str,
    ) -> bool;
}

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema)),
    // TODO: content rename?
    serde(tag = "as", content = "data", rename_all = "snake_case")
)]
// TODO: arbitrary
// TODO: derives
pub enum WalletOffchainInput {
    AsSelf {
        msg: WalletOffchainMessage,
        proof: String,
    },
    AsExtension {
        account_id: AccountId,
        input: String,
        output: String,
    },
}

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
// TODO: arbitrary
// TODO: derives
pub struct WalletOffchainMessage {
    pub path: Vec<AccountId>,
    pub signer_id: AccountId,
    pub chain_id: ChainId,
    // TODO: direction?
    pub msg: OffchainMessage,
}

// // TODO: docs
// #[cfg_attr(
//     feature = "serde",
//     derive(::serde::Serialize, ::serde::Deserialize),
//     cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
// )]
// #[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct WalletOffchainMessage {
//     pub output: String,

//     // TODO: maybe only if self?
//     pub msg: OffchainMessage,

//     #[cfg_attr(
//         feature = "serde",
//         serde(default, skip_serializing_if = "Option::is_none")
//     )]
//     pub as_extension_id: Option<AccountId>,

//     pub proof: Proof,
// }

// impl WalletOffchainMessage {
//     #[inline]
//     pub fn as_self(proof: impl Into<Proof>) -> Self {
//         Self {
//             as_extension_id: None,
//             proof: proof.into(),
//         }
//     }

//     #[inline]
//     pub fn as_extension(extension_id: impl Into<AccountId>, proof: impl Into<Proof>) -> Self {
//         Self {
//             as_extension_id: Some(extension_id.into()),
//             proof: proof.into(),
//         }
//     }

//     // TODO: naming
//     #[cfg(feature = "json")]
//     #[inline]
//     pub fn wrap_as_extension(self, extension_id: impl Into<AccountId>) -> Self {
//         Self::as_extension(
//             extension_id,
//             serde_json::to_string(&self).expect("JSON: failed to serialize"),
//         )
//     }
// }
