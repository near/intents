use defuse_borsh_utils::adapters::{As, TimestampSeconds};
use defuse_deadline::Deadline;
use near_sdk::{AccountId, borsh, env, near};

use crate::Request;

// TODO: versioned?
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedRequest {
    /// MUST be equal to chain_id of the network where the wallet-contract
    /// is deployed.
    pub chain_id: String,

    /// MUST be equal to the account_id of the wallet-contract.
    pub signer_id: AccountId,

    pub seqno: u32,

    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        borsh(
            serialize_with = "As::<TimestampSeconds<u32>>::serialize",
            deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
            schema(with_funcs(
                definitions = "As::<TimestampSeconds<u32>>::add_definitions_recursively",
                declaration = "As::<TimestampSeconds<u32>>::declaration",
            ),)
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

    // TODO: hash request or entire SignedRequestBody??? or both???
    // TODO: is it a request_id?
    // #[serde_as(as = "Hex")] // TODO: or base58?
    pub request: Request,
}

impl SignedRequest {
    const DOMAIN_PREFIX: &[u8] = b"NEAR_WALLET_CONTRACT";
    const DOMAIN_VERSION: &[u8] = b"1.0.0";

    pub fn hash(&self) -> [u8; 32] {
        // TODO: sha256 or keccak 256?
        env::keccak256_array(self.prehash())
    }

    fn prehash(&self) -> Vec<u8> {
        // TODO: Add NEP-461 tag prefix? - that should be a part of `proof`
        // envelope
        // TODO: hash `request` first, so that it might be possible to sign
        // for very limited-memory devices to sign at least hash
        // TODO: hashing the request is also better, because the request
        // hash becomes request_id...
        // TODO: should we include query_id in the request?...
        borsh::to_vec(&(Self::DOMAIN_PREFIX, Self::DOMAIN_VERSION, self))
            .unwrap_or_else(|_| unreachable!())
    }
}

// pub struct Domain<'a> {
//     pub name: Cow<'a, str>,
//     pub version: Cow<'a, str>,
// }

// pub trait Domain {}

// pub enum Envelope {
//     V1 {},
// }

// pub struct EnvelopeV1<'a> {
//     pub domain_name: Cow<'a, str>,
//     pub domain_version: u8,
// }

// pub struct ToSignV1 {
//     /// MUST be equal to the account_id of the wallet-contract.
//     pub signer_id: AccountId,

//     /// MUST be equal to chain_id of the network where the wallet-contract
//     /// is deployed.
//     pub chain_id: String,

//     pub seqno: u32,

//     #[cfg_attr(
//         all(feature = "abi", not(target_arch = "wasm32")),
//         borsh(
//             serialize_with = "As::<TimestampSeconds<u32>>::serialize",
//             deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
//             schema(with_funcs(
//                 definitions = "As::<TimestampSeconds<u32>>::add_definitions_recursively",
//                 declaration = "As::<TimestampSeconds<u32>>::declaration",
//             ),)
//         )
//     )]
//     #[cfg_attr(
//         any(not(feature = "abi"), target_arch = "wasm32"),
//         borsh(
//             serialize_with = "As::<TimestampSeconds<u32>>::serialize",
//             deserialize_with = "As::<TimestampSeconds<u32>>::deserialize",
//         )
//     )]
//     pub valid_until: Deadline,
// }
