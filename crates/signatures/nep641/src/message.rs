use defuse_time::Timestamp;
use near_account_id::AccountId;

#[cfg(feature = "borsh")]
use ::{defuse_borsh_utils::As, defuse_time::borsh::TimestampNanoSeconds};
#[cfg(feature = "arbitrary")]
use defuse_time::arbitrary::RangeNanos;
/// An offchain [authorization](OffchainAuthorization) message.
///
// TODO:
/// The implementation MUST verify given `proof` over offchain message, including
/// all of its fields, and return a list of "pending authorizations" that need to be
/// [resolved](crate::resolver::OffchainResolver::resolve_auth) on other accounts.
///
/// TODO: returned auths fields
///
/// The implementation MUST panic if:
/// * [`chain_id`](field@crate::OffchainMessage::chain_id) doesn't match
///   `env::chain_id()`
/// * [`resolver_id`](field@crate::OffchainMessage::resolver_id) doesn't
///   match `env::current_account_id()`
/// * `proof` is invalid for given [`OffchainMessage`](crate::OffchainMessage)
///
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
    // TODO: deny unknown fields?
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
// TODO: versioned?
pub struct OffchainMessage {
    // TODO: docs: direction?
    // TODO: is it a good fields order for HW wallets?
    // TODO: is path alone enough?
    /// Path starting from top-level authorizing account ID.
    pub path: Vec<AccountId>,

    /// Signer ID.
    ///
    /// The [resolver](crate::AuthResolver::w_resolve_auth) contract MUST panic if it doesn't
    /// match its current account ID.
    pub signer_id: AccountId,

    /// Chain ID.
    ///
    /// The [resolver](crate::AuthResolver::w_resolve_auth) contract MUST panic if it doesn't
    /// match its chain ID.
    pub chain_id: String,

    /// UNIX timestamp at the time of signing.
    ///
    /// The [resolver](crate::AuthResolver::w_resolve_auth) contract MUST panic if the timestamp
    /// is from the future. The contract MAY also panic if it performs some additional checks,
    /// such as TTL.
    ///
    /// Clients are recommended to set it slightly (e.g. 15 seconds) before the actual time of
    /// signing, so that it doesn't fail if the message gets resolved too fast.
    #[cfg_attr(
        feature = "arbitrary",
        arbitrary(with = ::arbitrary_with::As::<RangeNanos::<0>>::arbitrary),
    )]
    #[cfg_attr(
        feature = "borsh",
        borsh(
            serialize_with = "As::<TimestampNanoSeconds<u64>>::serialize",
            deserialize_with = "As::<TimestampNanoSeconds<u64>>::deserialize",
        ),
        cfg_attr(
            feature = "borsh-schema",
            borsh(schema(with_funcs(
                definitions = "As::<TimestampNanoSeconds<u64>>::add_definitions_recursively",
                declaration = "As::<TimestampNanoSeconds<u64>>::declaration",
            )))
        )
    )]
    pub timestamp: Timestamp,

    // TODO: domain
    // TODO: schema?

    // TODO: how to propagate domain and purpose/action?
    /// The actual signed payload
    /// TODO: docs
    pub payload: String,
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

    /// Replace [`resolver_id`](field@Self::resolver_id) with given account ID
    // /// on this message.
    // #[must_use]
    // #[inline]
    // pub fn with_resolver_id(mut self, resolver_id: impl Into<AccountId>) -> Self {
    //     self.resolver_id = resolver_id.into();
    //     self
    // }

    /// Returns whether this message is a top-level authorization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_nep641::OffchainMessage;
    /// let top_level = OffchainMessage {
    ///     signer_id: "wallet.near".parse().unwrap(),
    ///     resolver_id: "wallet.near".parse().unwrap(),
    ///     chain_id: "mainnet".to_string(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    /// assert!(top_level.is_top_level());
    ///
    /// let sub_auth = OffchainMessage {
    ///     signer_id: "wallet.near".parse().unwrap(),
    ///     resolver_id: "extension.near".parse().unwrap(),
    ///     chain_id: "mainnet".to_string(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    /// assert!(!sub_auth.is_top_level());
    /// ```
    #[inline]
    pub const fn is_top_level(&self) -> bool {
        self.path.is_empty()
    }

    // TODO: docs, naming, examples
    #[inline]
    pub const fn effective_account_id(&self) -> &AccountId {
        let Some(account_id) = self.path.as_slice().first() else {
            return &self.signer_id;
        };

        &account_id
    }

    /// Returns canonical hash of this offchain message, calculated as:
    ///
    /// ```text
    /// SHA3_256(b"NEAR_NEP641_OFFCHAIN_MESSAGE/V1" || borsh(msg))
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_nep641::OffchainMessage;
    /// # use hex_literal::hex;
    /// let msg = OffchainMessage {
    ///     signer_id: "wallet.near".parse().unwrap(),
    ///     resolver_id: "wallet.near".parse().unwrap(),
    ///     chain_id: "mainnet".to_string(),
    ///     payload: "Hello, Near!".to_string(),
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

// TODO
pub struct Message {
    pub domain: String,
    pub action: String,
    pub payload: String,
}
