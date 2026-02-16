pub mod borsh;
#[cfg(feature = "ed25519")]
pub mod ed25519;
pub mod hash;
pub mod no_sign;
#[cfg(feature = "webauthn")]
pub mod webauthn;

use std::borrow::Cow;

use defuse_borsh_utils::adapters::{As, TimestampSeconds};
pub use defuse_deadline::Deadline;
use near_sdk::{AccountId, CryptoHash, env, near};

use crate::Request;

pub use self::{borsh::*, hash::*};

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
    /// Domain prefix for the wallet-contract
    pub const DOMAIN_PREFIX: &str = "NEAR_WALLET_CONTRACT";

    /// Request hash
    pub fn hash(&self) -> CryptoHash {
        let serialized = ::near_sdk::borsh::to_vec(&self).unwrap_or_else(|_| unreachable!());

        env::sha256_array(serialized)
    }

    /// Prefixes the request message with domain for signing/verification
    pub fn with_domain(&self) -> SignatureDomain<'static, WalletDomain<'_>> {
        SignatureDomain::new(Self::DOMAIN_PREFIX, WalletDomain::V1(Cow::Borrowed(self)))
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureDomain<'a, T> {
    pub domain: Cow<'a, str>,
    #[serde(flatten)]
    pub data: T,
}

impl<'a, T> SignatureDomain<'a, T> {
    pub fn new(domain: impl Into<Cow<'a, str>>, data: T) -> Self {
        Self {
            domain: domain.into(),
            data,
        }
    }
}

/// Version of wallet-contract domain
#[near(serializers = [borsh(use_discriminant = true), json])]
#[serde(tag = "version", content = "message", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum WalletDomain<'a> {
    V1(Cow<'a, RequestMessage>) = 0,
}
