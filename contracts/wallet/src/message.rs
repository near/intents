use core::time::Duration;

use defuse_time::Timestamp;
use near_account_id::AccountId;

use crate::request::Request;

#[cfg(feature = "borsh")]
use ::{
    defuse_borsh_utils::{As, DurationSeconds as BorshDurationSeconds},
    defuse_time::borsh::TimestampNanoSeconds,
};
#[cfg(feature = "arbitrary")]
use defuse_time::arbitrary::RangeNanos;
#[cfg(feature = "serde")]
use serde_with::DurationSeconds;

/// Domain prefix for signing [`RequestMessage`].
///
/// This prefix doesn't break NEP-461 assumptions, since first four bytes
/// borsh-deserialize to `1380009294u32`, which is in `[1 << 30, 1 << 31)`
/// range for on-chain messages.
pub const WALLET_DOMAIN: &[u8] = b"NEAR_WALLET_CONTRACT/V1";

/// Signable message containing [`Request`] to execute via
/// [`w_execute_signed()`](crate::contract::Wallet::w_execute_signed) contract
/// method.
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestMessage {
    /// Chain id (e.g. `mainnet`).
    /// MUST be equal to `chain_id` of the network.
    pub chain_id: String,

    /// Signer id.
    /// MUST be equal to the `AccountId` of the wallet-contract instance.
    pub signer_id: AccountId,

    /// A non-sequential `timeout`-bounded nonce for this request.
    ///
    /// # Optimal Order
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
    /// nonces or at least, generate them semi-sequentially (i.e. where the
    /// nonce is randomized after each 32 sequential ones) to reduce storage
    /// usage and, hopefully, fit into ZBA limits.
    pub nonce: u32,

    #[cfg_attr(
        feature = "arbitrary",
        arbitrary(with = ::arbitrary_with::As::<RangeNanos::<0>>::arbitrary),
    )]
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
    /// # Optimal lag
    ///
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
    pub timeout: Duration,

    /// Request to execute
    pub request: Request,
}

impl RequestMessage {
    /// Returns canonical hash of the request message:
    ///
    /// ```text
    /// SHA3-256(b"NEAR_WALLET_CONTRACT/V1" || borsh(msg))
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use core::time::Duration;
    /// # use defuse_wallet::{Request, RequestMessage, Timestamp};
    /// # use hex_literal::hex;
    /// let msg = RequestMessage {
    ///     chain_id: "mainnet".to_string(),
    ///     signer_id: "0s0000000000000000000000000000000000000000".parse().unwrap(),
    ///     nonce: 0,
    ///     created_at: Timestamp::UNIX_EPOCH,
    ///     timeout: Duration::from_secs(3600),
    ///     request: Request::new(),
    /// };
    ///
    /// assert_eq!(
    ///     msg.hash(),
    ///     hex!("e42ac706e27f0157624ee49fc4693c9cc9666c5e51358b7d57f79ee16005ded7"),
    /// );
    /// ```
    #[cfg(all(feature = "digest", feature = "borsh"))]
    pub fn hash(&self) -> [u8; 32] {
        use defuse_digest::{Digest, sha3::Sha3_256};
        use digest_io::IoWrapper;

        let mut hasher = IoWrapper(Sha3_256::new_with_prefix(WALLET_DOMAIN));
        // serialize directly to hasher
        ::borsh::to_writer(&mut hasher, self).expect("borsh: failed to serialize");

        hasher.0.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    // https://nearblocks.io/txns/6vytw7NgAiPkJ3KYAyt18es4mDnwZ8knjpB7LHVJejAL
    #[case(
        r#"{"nonce":2845491008,"request":{"external":[{"actions":[{"action":"function_call","payload":{"args":"eyJyZXF1ZXN0Ijp7InBheWxvYWRfdjIiOnsiRWNkc2EiOiIwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwIn0sImRvbWFpbl9pZCI6MCwicGF0aCI6IiJ9fQ==","deposit":"1","function_name":"sign"}}],"receiver_id":"v1.signer"}]},"chain_id":"mainnet","signer_id":"0se5eba21e8f191e1880e453794bc551dfa50a3419","created_at":"2026-07-07T11:13:29Z","timeout_secs":3600}"#,
        hex!("06f269191431372337a0c606a15822e349bd0d5ec317704f97bef1a4ed6f5e1d"),
    )]
    fn json_hash(#[case] json: &str, #[case] hash: [u8; 32]) {
        let msg: RequestMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.hash(), hash);
    }
}
