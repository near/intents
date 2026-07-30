use near_account_id::AccountId;

/// Chain ID (e.g. `mainnet`)
pub type ChainId = String;
/// Mainnet [chain ID](ChainId).
pub const MAINNET: &str = "mainnet";

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
    /// _Effective_ signer ID, i.e. top-level account ID for the whole
    /// authorization.
    ///
    // TODO
    /// MUST be equal to
    pub signer_id: AccountId,

    /// _Real_ resolver ID.
    ///
    /// MUST be equal to account ID of [resolving](crate::contract::OffchainAuthorizer::w_resolve_auth) contract.
    // TODO: resolver_ids?
    pub resolver_ids: Vec<AccountId>,

    /// Chain ID.
    ///
    /// MUST be equal to chain ID of [verifying](crate::contract::OffchainAuthorizer::w_resolve_auth) contract.
    pub chain_id: ChainId,

    // TODO: domain
    // TODO: schema?
    // TODO: deadline like in TON Connect?
    pub msg: String,
    // TODO: "verify that account A has >= 1000 tokens B", "owns NFT", etc..
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
    pub const DOMAIN: &[u8] = b"NEAR_NEP641_OFFCHAIN_MESSAGE/V1";

    #[inline]
    pub const fn resolver_id(&self) -> &AccountId {
        if let Some(resolver_id) = self.resolver_ids.as_slice().last() {
            return resolver_id;
        }
        &self.signer_id
    }

    #[inline]
    pub fn with_downstream_resolver(mut self, resolver_id: impl Into<AccountId>) -> Self {
        self.resolver_ids.push(resolver_id.into());
        self
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
    /// use defuse_nep641::{OffchainMessage, MAINNET};
    /// # use hex_literal::hex;
    ///
    /// let msg = OffchainMessage {
    ///     signer_id: "signer.near".parse().unwrap(),
    ///     chain_id: MAINNET.to_string(),
    ///     msg: "some message".to_string(),
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

        let mut hasher = IoWrapper(Sha3_256::new_with_prefix(Self::DOMAIN));
        // serialize directly to hasher
        ::borsh::to_writer(&mut hasher, self).expect("borsh: failed to serialize");

        hasher.0.finalize().into()
    }
}

/// TODO: docs
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
// TODO: no borsh?
// #[cfg_attr(
//     feature = "borsh",
//     derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
//     cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
// )]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PendingAuthorization {
    pub resolver_id: AccountId,
    pub proof: Proof,
}
