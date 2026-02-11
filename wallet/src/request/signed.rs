use std::borrow::Cow;

use defuse_borsh_utils::adapters::{As, TimestampSeconds};
use defuse_deadline::Deadline;
use near_sdk::{AccountId, borsh, env, near};

use crate::Request;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedRequest {
    /// MUST be equal to chain_id of the network where the wallet-contract
    /// is deployed.
    pub chain_id: String,

    /// MUST be equal to the AccountId of the wallet-contract executing
    /// this signed request.
    pub signer_id: AccountId,

    /// MUST be equal to the current seqno on the contract.
    pub seqno: u32,

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

impl SignedRequest {
    pub const DOMAIN_PREFIX: &str = "NEAR_WALLET_CONTRACT";

    pub fn hash(&self) -> [u8; 32] {
        // TODO: sha256 or keccak256?
        env::keccak256_array(self.prehash())
    }

    fn prehash(&self) -> Vec<u8> {
        // TODO: hash `request` first, so that it might be possible to sign
        // for very limited-memory devices to sign at least the hash?
        borsh::to_vec(&self.to_domain()).unwrap_or_else(|_| unreachable!())
    }

    pub fn to_domain(&self) -> SignatureDomain<'static, WalletDomain<'_>> {
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

#[near(serializers = [borsh(use_discriminant = true), json])]
#[serde(tag = "version", content = "message", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum WalletDomain<'a> {
    V1(Cow<'a, SignedRequest>) = 0,
}
