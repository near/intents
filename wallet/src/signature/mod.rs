mod borsh;
pub mod domain;
#[cfg(feature = "ed25519")]
pub mod ed25519;
mod hash;
pub mod no_sign;
#[cfg(feature = "webauthn")]
pub mod webauthn;

use defuse_borsh_utils::adapters::{As, TimestampSeconds};
pub use defuse_deadline::Deadline;
use near_sdk::{AccountId, CryptoHash, env, near};

use crate::Request;

pub use self::{borsh::*, domain::*, hash::*};

/// Signing standard, which defines the public key and how `signature` on
/// `msg` is verified.
pub trait SigningStandard<M> {
    type PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool;
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestMessage {
    /// MUST be equal to `chain_id` of the network where the wallet-contract
    /// is deployed.
    pub chain_id: String,

    /// MUST be equal to the AccountId of the wallet-contract executing
    /// this signed request.
    pub signer_id: AccountId,

    /// MUST be equal to the current seqno on the contract.
    pub seqno: u32,

    /// The deadline for this signed request.
    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<TimestampSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<TimestampSeconds<u32>>::declaration",
            ))
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

    pub request: Request,
}

impl RequestMessage {
    /// Request hash
    pub fn hash(&self) -> CryptoHash {
        let serialized = ::near_sdk::borsh::to_vec(&self).unwrap_or_else(|_| unreachable!());

        env::sha256_array(serialized)
    }
}
