mod borsh;
mod domain;
#[cfg(feature = "ed25519")]
pub mod ed25519;
mod hash;
pub mod no_sign;

#[cfg(feature = "webauthn")]
pub mod webauthn;

pub use defuse_deadline::Deadline;
use near_sdk::{AccountId, CryptoHash, borsh::BorshSerialize, env, near};

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
pub struct RequestMessage<Nonce> {
    /// MUST be equal to `chain_id` of the network where the wallet-contract
    /// is deployed.
    pub chain_id: String,

    /// MUST be equal to the AccountId of the wallet-contract executing
    /// this signed request.
    pub signer_id: AccountId,

    // TODO: docs
    // TODO: serde/borsh?
    #[serde(flatten)]
    pub nonce: Nonce,

    pub request: Request,
}

impl<Nonce> RequestMessage<Nonce>
where
    Nonce: BorshSerialize,
{
    /// Request hash
    pub fn hash(&self) -> CryptoHash {
        let serialized = ::near_sdk::borsh::to_vec(&self).unwrap_or_else(|_| unreachable!());

        env::sha256_array(serialized)
    }
}
