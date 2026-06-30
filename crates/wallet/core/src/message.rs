use core::time::Duration;

use defuse_time::Timestamp;
use near_account_id::AccountId;

use crate::request::Request;

#[cfg(feature = "borsh")]
use ::{
    defuse_borsh_utils::{As, DurationSeconds as BorshDurationSeconds},
    defuse_time::borsh::TimestampNanoSeconds,
};
#[cfg(feature = "serde")]
use serde_with::DurationSeconds;

pub const WALLET_DOMAIN: &[u8] = b"NEAR_WALLET_CONTRACT/V1";

// clippy seems to have false-positive caused by `thread_local!()`
// usage in `hash()` method
#[allow(clippy::unsafe_derive_deserialize)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestMessage {
    /// Chain id (e.g. `mainnet`).
    /// MUST be equal to `chain_id` of the network.
    pub chain_id: String,

    /// Signer id.
    /// MUST be equal to the `AccountId` of the wallet-contract instance.
    pub signer_id: AccountId,

    /// A non-sequential `timeout`-bounded nonce for this request.
    ///
    /// NOTE:
    ///
    /// Since nonces are non-sequential, the contract needs to keep track of
    /// used ones, which causes the storage to grow. Each nonce is stored for
    /// at most `2 * timeout` and then cleaned up.
    ///
    /// Nonces are stored in bitmap represented as key-value mapping where
    /// the key 27 is bits long and the value is 32 bits long. First 27 bits
    /// of `nonce` are used as the key, while the last 5 bits denote the bit
    /// position that needs to be set in the corresponding value.
    ///
    /// As a result, clients are recommended to use incrementing counter for
    /// nonces or at least, generate them semi-sequentially to reduce storage
    /// usage and, hopefully, fit into ZBA limits. See
    /// [`ConcurrentNonces`](crate::ConcurrentNonces) implementation.
    pub nonce: u32,

    #[cfg_attr(
        feature = "borsh-schema",
        borsh(
            serialize_with = "As::<TimestampNanoSeconds<u64>>::serialize",
            deserialize_with = "As::<TimestampNanoSeconds<u64>>::deserialize",
            schema(with_funcs(
                definitions = "As::<TimestampNanoSeconds<u64>>::add_definitions_recursively",
                declaration = "As::<TimestampNanoSeconds<u64>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        all(feature = "borsh", not(feature = "borsh-schema")),
        borsh(
            serialize_with = "As::<TimestampNanoSeconds<u64>>::serialize",
            deserialize_with = "As::<TimestampNanoSeconds<u64>>::deserialize",
        )
    )]
    /// Timestamp when this request was created (in RFC-3339 format).
    ///
    /// NOTE:
    /// The contract ensures that `now() - timeout <= created_at <= now()`,
    /// where `now()` is the current block timestamp. Due to the desentralized
    /// nature of consensus in blockchains, block timestamps usually lag a
    /// bit behind the actual time when it's produced. As a result, clients
    /// are recommended to set `created_at` slightly (e.g. 60 seconds) before
    /// the actual time of signing, so that it doesn't fail on-chain if it
    /// arrives too fast.
    pub created_at: Timestamp,

    #[cfg_attr(
        feature = "borsh-schema",
        borsh(
            serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
            deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<BorshDurationSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<BorshDurationSeconds<u32>>::declaration",
            ))
        )
    )]
    #[cfg_attr(
        all(feature = "borsh", not(feature = "borsh-schema")),
        borsh(
            serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
            deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
        )
    )]
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "DurationSeconds"),
        serde(rename = "timeout_secs")
    )]
    /// Maximum timeout for validity of this request after `created_at`.
    /// The actual timeout for the request is `min(msg.timeout, contract.timeout)`
    /// to prevent replay attacks.
    /// See [`w_timeout_secs()`](crate::Wallet::w_timeout_secs).
    pub timeout: Duration,

    /// Request to execute
    pub request: Request,
}

impl RequestMessage {
    /// Returns canonical hash of the request message
    #[cfg(all(feature = "digest", feature = "borsh"))]
    pub fn hash(&self) -> [u8; 32] {
        use defuse_digest::{Digest, sha3::Sha3_256};
        use digest_io::IoWrapper;

        thread_local! {
            // per-thread lazily-initialized hasher with pre-processed prefix
            static HASHER: Sha3_256 = Sha3_256::new_with_prefix(WALLET_DOMAIN);
        }

        let mut hasher = IoWrapper(HASHER.with(Clone::clone));
        // serialize directly to hasher
        ::borsh::to_writer(&mut hasher, self).expect("borsh: failed to serialize");

        hasher.0.finalize().into()
    }
}
