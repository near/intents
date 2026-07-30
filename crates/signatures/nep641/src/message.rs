use near_account_id::AccountId;

/// Chain ID (e.g. `mainnet`)
pub type ChainId = String;

/// A proof for [`OffchainAuthorization`]
pub type Proof = String;

// /// An authorization to be [resolved](crate::resolver::OffchainResolver::resolve_auth)
// /// **offchain**.
// #[cfg_attr(
//     feature = "serde",
//     derive(::serde::Serialize, ::serde::Deserialize),
//     cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
// )]
// #[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
// #[cfg_attr(
//     feature = "borsh",
//     derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
//     cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
// )]
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct OffchainAuthorization {
//     /// A message to verify [`proof`](field@Self::proof) over.
//     pub msg: OffchainMessage,

//     /// A proof over [`msg`](field@Self::msg).
//     ///
//     /// MUST account for **all** fields of [`OffchainMessage`].
//     pub proof: Proof,
// }

/// An offchain [authorization](OffchainAuthorization) message.
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OffchainMessage {
    // TODO: versioned?
    /// Signer ID.
    ///
    /// MUST be equal to account ID of [verifying](crate::contract::AuthResolver::w_resolve_auth) contract.
    pub signer_id: AccountId,

    /// Account ID which the top-level authorization is intended to be
    /// [resolved](crate::contract::AuthResolver::w_resolve_auth) for.
    ///
    /// If this authorization is top-level itself, then this field MUST match
    /// `field@Self::signer_id`.
    pub sign_for: AccountId,

    /// Chain ID.
    ///
    /// MUST be equal to chain ID of [verifying](crate::contract::AuthResolver::w_resolve_auth) contract.
    pub chain_id: ChainId,

    // TODO: domain
    // TODO: schema?
    // TODO: deadline like in TON Connect?
    pub msg: String,
}

impl OffchainMessage {
    // TODO: NEP-461 compatibility?
    /// A prefix used for [canonical hash](Self::hash).
    ///
    /// This prefix doesn't break NEP-461 assumptions, since first four bytes
    /// borsh-deserialize to `1380009294u32`, which is in `[1 << 30, 1 << 31)`
    /// range for on-chain messages.
    // TODO: "on/off chain"
    // TODO: rename to PREFIX?
    pub const DOMAIN_SEPARATOR: &[u8] = b"NEAR_NEP641_OFFCHAIN_MESSAGE/V1";

    /// Replace [`signer_id`](field@Self::signer_id) with given account ID
    /// on this message.
    #[inline]
    pub fn with_signer_id(mut self, signer_id: impl Into<AccountId>) -> Self {
        self.signer_id = signer_id.into();
        self
    }

    /// Returns whether this message is a top-level authorization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_nep641::OffchainMessage;
    /// let top_level = OffchainMessage {
    ///     signer_id: "wallet.near".parse().unwrap(),
    ///     sign_for: "wallet.near".parse().unwrap(),
    ///     chain_id: "mainnet".to_string(),
    ///     msg: "Hello, world!".to_string(),
    /// };
    /// assert!(top_level.is_top_level());
    ///
    /// let sub_auth = OffchainMessage {
    ///     signer_id: "extension.near".parse().unwrap(),
    ///     sign_for: "wallet.near".parse().unwrap(),
    ///     chain_id: "mainnet".to_string(),
    ///     msg: "Hello, world!".to_string(),
    /// };
    /// assert!(!sub_auth.is_top_level());
    /// ```
    #[inline]
    pub fn is_top_level(&self) -> bool {
        self.sign_for == self.signer_id
    }

    /// Returns canonical hash of this offchain message:
    /// TODO
    /// ```text
    /// SHA3_256()
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_nep641::OffchainMessage;
    /// # use hex_literal::hex;
    /// let msg = OffchainMessage {
    ///     signer_id: "wallet.near".parse().unwrap(),
    ///     sign_for: "wallet.near".parse().unwrap(),
    ///     chain_id: "mainnet".to_string(),
    ///     msg: "Hello, world!".to_string(),
    /// };
    ///
    /// assert_eq!(
    ///     msg.hash(),
    ///     hex!(""),
    /// );
    /// ```
    #[cfg(all(feature = "digest", feature = "borsh"))]
    #[inline]
    pub fn hash(&self) -> [u8; 32] {
        use defuse_digest::{Digest, sha3::Sha3_256};
        use digest_io::IoWrapper;

        let mut hasher = IoWrapper(Sha3_256::new_with_prefix(Self::DOMAIN_SEPARATOR));
        // serialize directly to hasher
        ::borsh::to_writer(&mut hasher, self).expect("borsh: failed to serialize");

        hasher.0.finalize().into()
    }
}

// /// TODO: docs
// #[cfg_attr(
//     feature = "serde",
//     derive(::serde::Serialize, ::serde::Deserialize),
//     cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
// )]
// #[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
// // TODO: no borsh?
// // #[cfg_attr(
// //     feature = "borsh",
// //     derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
// //     cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
// // )]
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct PendingAuthorization {
//     pub resolver_id: AccountId,
//     pub proof: Proof,
// }
