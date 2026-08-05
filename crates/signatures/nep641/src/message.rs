use defuse_time::Timestamp;
use near_account_id::AccountId;

#[cfg(feature = "borsh")]
use ::{defuse_borsh_utils::As, defuse_time::borsh::TimestampNanoSeconds};
#[cfg(feature = "arbitrary")]
use defuse_time::arbitrary::RangeNanos;

/// Signable offchain message.
///
/// The [verifying](crate::AuthResolver::w_resolve_auth) contract MUST verify a signature over
/// this entire message, including **all** of its fields, and panic if the signature is invalid.
///
/// See the documentation of each individual field for additional requirements.
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
    /// Chain ID.
    ///
    /// The [verifying](crate::AuthResolver::w_resolve_auth) contract MUST panic if it doesn't
    /// match its chain ID.
    pub chain_id: String,

    /// Signer ID.
    ///
    /// The [verifying](crate::AuthResolver::w_resolve_auth) contract MUST panic if it doesn't
    /// match its current account ID.
    pub signer_id: AccountId,

    /// Path to the top-level [resolver](crate::AuthResolver) ID.
    ///
    /// The path is oriented "bottom-top", where the parent resolver ID is the _first_ element and
    /// the top-level resolver ID is the _last_ one. Empty path means that this message is a
    /// top-level authorization itself.
    ///
    /// The verifying contract MUST panic if it doesn't match the **REVERSED** `path` passed as
    /// an argument to [`w_resolve_auth()`](crate::AuthResolver::w_resolve_auth) method.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub path: Vec<AccountId>,

    /// UNIX timestamp at the time of signing.
    ///
    /// The [verifying](crate::AuthResolver::w_resolve_auth) contract MUST panic if the timestamp
    /// is later than the current block timestamp. This prevents an offchain resolver from
    /// validating this authorization against chain state from the past, which may also happen
    /// unintentionally when its RPC endpoint lags behind the tip of the network.
    ///
    /// The contract MAY also panic if it performs some additional checks, such as TTL.
    ///
    /// Clients are recommended to set it slightly (e.g. 60 seconds) before the actual signing
    /// time to accommodate clock skew and the normal lag of block timestamps.
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

    /// The authorized payload.
    ///
    /// dApps and protocols are recommended to use human-readable
    /// [`JsonPayload`] format for user-facing interactions and maximum
    /// compatibility across wallets.
    pub payload: String,
}

impl OffchainMessage {
    /// A prefix used for [canonical hash](Self::hash).
    pub const DOMAIN_SEPARATOR: &[u8] = b"NEAR_NEP641_OFFCHAIN_MESSAGE/V1";

    /// Returns whether this message is a top-level authorization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_nep641::{OffchainMessage, Timestamp};
    /// let msg = OffchainMessage {
    ///     chain_id: "mainnet".to_string(),
    ///     signer_id: "wallet.near".parse().unwrap(),
    ///     path: [].into(),
    ///     timestamp: Timestamp::now(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    /// assert!(msg.is_top_level());
    ///
    /// let msg = OffchainMessage {
    ///     chain_id: "mainnet".to_string(),
    ///     signer_id: "wallet.near".parse().unwrap(),
    ///     path: vec!["v1.signer".parse().unwrap()],
    ///     timestamp: Timestamp::now(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    /// assert!(!msg.is_top_level());
    /// ```
    #[inline]
    pub const fn is_top_level(&self) -> bool {
        self.path.as_slice().is_empty()
    }

    /// Returns depth of this authorization, i.e. how many hops away the top-level resolver ID is.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_nep641::{OffchainMessage, Timestamp};
    /// let msg = OffchainMessage {
    ///     chain_id: "mainnet".to_string(),
    ///     signer_id: "wallet.near".parse().unwrap(),
    ///     path: [].into(),
    ///     timestamp: Timestamp::now(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    /// assert_eq!(msg.depth(), 0);
    ///
    /// let msg = OffchainMessage {
    ///     chain_id: "mainnet".to_string(),
    ///     signer_id: "extension.near".parse().unwrap(),
    ///     path: vec![
    ///         "wallet.near".parse().unwrap(),
    ///         "v1.signer".parse().unwrap(),
    ///     ],
    ///     timestamp: Timestamp::now(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    /// assert_eq!(msg.depth(), 2);
    /// ```
    #[inline]
    pub const fn depth(&self) -> usize {
        self.path.as_slice().len()
    }

    /// Returns a top-level resolver account ID that this message authorizes the payload for.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_nep641::{OffchainMessage, Timestamp};
    /// let msg = OffchainMessage {
    ///     chain_id: "mainnet".to_string(),
    ///     signer_id: "wallet.near".parse().unwrap(),
    ///     path: [].into(),
    ///     timestamp: Timestamp::now(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    /// assert_eq!(msg.top_level_id(), "wallet.near");
    ///
    /// let msg = OffchainMessage {
    ///     chain_id: "mainnet".to_string(),
    ///     signer_id: "extension.near".parse().unwrap(),
    ///     path: vec![
    ///         "wallet.near".parse().unwrap(),
    ///         "v1.signer".parse().unwrap(),
    ///     ],
    ///     timestamp: Timestamp::now(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    /// assert_eq!(msg.top_level_id(), "v1.signer");
    /// ```
    #[inline]
    pub const fn top_level_id(&self) -> &AccountId {
        if let Some(top_level) = self.path.as_slice().last() {
            return top_level;
        }
        &self.signer_id
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
    ///     chain_id: "mainnet".to_string(),
    ///     signer_id: "extension.near".parse().unwrap(),
    ///     path: vec!["wallet.near".parse().unwrap()],
    ///     timestamp: "2026-08-05T07:28:00.123456789Z".parse().unwrap(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    ///
    /// assert_eq!(
    ///     msg.hash(),
    ///     hex!("d98eb326e626ad325bfc4ff7d4e487d2c8cbd820a5a29683be6b931a3dd5a280"),
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

    /// Deterministically convert this message into NEP-413 payload, with optional callback URL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hex_literal::hex;
    /// use defuse_nep641::OffchainMessage;
    /// use defuse_nep413::Nep413Payload;
    ///
    /// let msg = OffchainMessage {
    ///     chain_id: "mainnet".to_string(),
    ///     signer_id: "extension.near".parse().unwrap(),
    ///     path: vec![
    ///         "wallet.near".parse().unwrap(),
    ///         "v1.signer".parse().unwrap(),
    ///     ],
    ///     timestamp: "2026-08-05T07:28:00.123456789Z".parse().unwrap(),
    ///     payload: "Hello, Near!".to_string(),
    /// };
    ///
    /// assert_eq!(
    ///     msg.into_nep413_payload("https://wallet.com/callback"),
    ///     Nep413Payload {
    ///         message: "Hello, Near!".to_string(),
    ///         nonce: hex!("d98eb326e626ad325bfc4ff7d4e487d2c8cbd820a5a29683be6b931a3dd5a280"),
    ///         recipient: "mainnet @ extension.near -> wallet.near -> v1.signer".to_string(),
    ///         callback_url: Some("https://wallet.com/callback".to_string()),
    ///     },
    /// );
    /// ```
    #[cfg(feature = "nep413")]
    pub fn into_nep413_payload(
        self,
        callback_url: impl Into<Option<String>>,
    ) -> ::defuse_nep413::Nep413Payload {
        use defuse_nep413::Nep413Payload;
        use itertools::Itertools;

        Nep413Payload {
            // bound full message contents via hash
            nonce: self.hash(),
            // `<chain_id> @ <signer_id>[ -> path]...`
            recipient: format!(
                "{} @ {}",
                self.chain_id,
                std::iter::once(&self.signer_id)
                    .chain(&self.path)
                    .join(" -> ")
            ),
            // the actual authorized payload
            message: self.payload,
            callback_url: callback_url.into(),
        }
    }
}

/// Domain-separated JSON [payload](field@OffchainMessage::payload).
#[cfg(feature = "json")]
#[cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ::serde::Serialize, ::serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonPayload {
    /// dApp or protocol domain, e.g. `near.com` or `Near MPC`.
    pub domain: String,

    /// An action to be taken on dApp/protocol, e.g. `Login` or `Sign`.
    pub action: String,

    /// A generic dApp/protocol-specific message in human-readable format, e.g. plain text or JSON.
    ///
    /// The message can contain arbitrary data, such as timestamps, deadlines, TTLs, nonces, etc...
    pub msg: String,
}

#[cfg(feature = "json")]
const _: () = {
    use core::str::FromStr;

    impl From<&JsonPayload> for String {
        #[inline]
        fn from(value: &JsonPayload) -> Self {
            // pretty JSON for better readability
            serde_json::to_string_pretty(value).expect("JSON")
        }
    }

    impl From<JsonPayload> for String {
        #[inline]
        fn from(value: JsonPayload) -> Self {
            (&value).into()
        }
    }

    impl FromStr for JsonPayload {
        type Err = serde_json::Error;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            serde_json::from_str(s)
        }
    }
};
