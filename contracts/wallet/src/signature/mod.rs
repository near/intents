mod borsh;
mod domain;
#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "eip712")]
pub mod eip712;
mod hash;
pub mod no_sign;

#[cfg(feature = "webauthn")]
pub mod webauthn;

use std::time::Duration;

use defuse_borsh_utils::adapters::{As, DurationSeconds as BorshDurationSeconds, TimestampSeconds};
pub use defuse_deadline::Deadline;
use near_sdk::{AccountId, CryptoHash, env, near, serde_with::DurationSeconds};

use crate::Request;

pub use self::{borsh::*, domain::*, hash::*};

/// Signing standard, which defines the public key and how `signature` on
/// `msg` is verified.
pub trait SigningStandard<M> {
    /// Public key used by the signing standard.
    type PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool;
}

#[near(serializers = [borsh, json])]
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
        not(feature = "abi"),
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
        )
    )]
    #[cfg_attr(
        feature = "abi",
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<TimestampSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<TimestampSeconds<u32>>::declaration",
            ))
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
    pub created_at: Deadline,

    #[cfg_attr(
        feature = "abi",
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
        not(feature = "abi"),
        borsh(
            serialize_with = "As::<BorshDurationSeconds<u32>>::serialize",
            deserialize_with = "As::<BorshDurationSeconds<u32>>::deserialize",
        )
    )]
    #[serde(rename = "timeout_secs")]
    #[serde_as(as = "DurationSeconds")]
    /// Maximum timeout for validity of this request after `created_at`.
    /// The actual timeout for the request is `min(msg.timeout, contract.timeout)`
    /// to prevent replay attacks.
    /// See [`w_timeout_secs()`](crate::Wallet::w_timeout_secs).
    pub timeout: Duration,

    /// Request to execute
    pub request: Request,
}

impl RequestMessage {
    /// Request hash
    pub fn hash(&self) -> CryptoHash {
        let serialized = ::near_sdk::borsh::to_vec(self).unwrap_or_else(|_| unreachable!());

        env::sha256_array(serialized)
    }
}
