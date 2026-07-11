use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use borsh::{BorshDeserialize, BorshSerialize};
use defuse_crypto::{
    ed25519::{Ed25519, Ed25519PublicKey},
    fmt::{ParseCurveError, TypedCurve, checked_base58_decode_array},
    p256::{P256, P256UncompressedPublicKey},
    secp256k1::{Secp256k1, Secp256k1UncompressedPublicKey},
};
use defuse_digest::{Digest, sha3::Keccak256};
use near_sdk::{AccountId, AccountIdRef};
use serde_with::{DeserializeFromStr, SerializeDisplay};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "abi", derive(::borsh::BorshSchema))]
#[derive(
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    SerializeDisplay,
    DeserializeFromStr,
    BorshSerialize,
    BorshDeserialize,
    derive_more::From,
)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum PublicKey {
    Ed25519(Ed25519PublicKey) = 0,
    Secp256k1(Secp256k1UncompressedPublicKey) = 1,
    P256(P256UncompressedPublicKey) = 2,
}

impl PublicKey {
    #[inline]
    pub fn to_implicit_account_id(&self) -> AccountId {
        match self {
            Self::Ed25519(pk) => {
                // https://docs.near.org/concepts/protocol/account-id#implicit-address
                hex::encode(pk)
            }
            Self::Secp256k1(pk) => {
                // https://ethereum.org/en/developers/docs/accounts/#account-creation
                format!("0x{}", hex::encode(&Keccak256::digest(pk)[12..32]))
            }
            Self::P256(pk) => {
                // In order to keep compatibility with all existing standards
                // within Near ecosystem (e.g. NEP-245), we need our implicit
                // account_ids to be fully backwards-compatible with Near's
                // implicit AccountId.
                //
                // To avoid introducing new implicit account id types, we
                // reuse existing Eth Implicit schema with same hash func.
                // To avoid collisions between addresses for different curves,
                // we add "p256" ("\x70\x32\x35\x36") prefix to the public key
                // before hashing.
                //
                // So, the final schema looks like:
                // "0x" .. hex(keccak256("p256" .. pk)[12..32])
                format!(
                    "0x{}",
                    hex::encode(
                        &Keccak256::new_with_prefix(b"p256")
                            .chain_update(pk)
                            .finalize()[12..32]
                    )
                )
            }
        }
        .try_into()
        .unwrap_or_else(|_| unreachable!())
    }

    #[inline]
    pub fn from_implicit_account_id(account_id: &AccountIdRef) -> Option<Self> {
        let mut pk = [0; 32];
        // Only NearImplicitAccount can be reversed
        hex::decode_to_slice(account_id.as_str(), &mut pk).ok()?;
        Some(Ed25519PublicKey(pk).into())
    }
}

impl Debug for PublicKey {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ed25519(pk) => pk.to_string(),
                Self::Secp256k1(pk) => pk.to_string(),
                Self::P256(pk) => pk.to_string(),
            }
        )
    }
}

impl Display for PublicKey {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for PublicKey {
    type Err = ParseCurveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (curve, data) = s
            .split_once(':')
            // ed25519 by default
            .unwrap_or((Ed25519::CURVE_TYPE, s));

        match curve {
            Ed25519::CURVE_TYPE => checked_base58_decode_array(data)
                .map(Ed25519PublicKey)
                .map(Into::into),
            Secp256k1::CURVE_TYPE => checked_base58_decode_array(data)
                .map(Secp256k1UncompressedPublicKey)
                .map(Into::into),
            P256::CURVE_TYPE => checked_base58_decode_array(data)
                .map(P256UncompressedPublicKey)
                .map(Into::into),
            _ => Err(ParseCurveError::WrongCurveType),
        }
    }
}

#[cfg(feature = "abi")]
const _: () = {
    use schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Metadata, Schema, SchemaObject},
    };

    impl JsonSchema for PublicKey {
        fn schema_name() -> String {
            String::schema_name()
        }

        fn is_referenceable() -> bool {
            false
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                extensions: std::iter::once(("contentEncoding", "base58".into()))
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
                metadata: Some(
                    Metadata {
                        examples: [
                            Self::example_ed25519(),
                            Self::example_secp256k1(),
                            Self::example_p256(),
                        ]
                        .map(serde_json::to_value)
                        .map(Result::unwrap)
                        .into(),
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            }
            .into()
        }
    }

    impl PublicKey {
        pub(super) fn example_ed25519() -> Self {
            "ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJugxm"
                .parse()
                .unwrap()
        }

        pub(super) fn example_secp256k1() -> Self {
            "secp256k1:3aMVMxsoAnHUbweXMtdKaN1uJaNwsfKv7wnc97SDGjXhyK62VyJwhPUPLZefKVthcoUcuWK6cqkSU4M542ipNxS3"
                .parse()
                .unwrap()
        }

        pub(super) fn example_p256() -> Self {
            "p256:3aMVMxsoAnHUbweXMtdKaN1uJaNwsfKv7wnc97SDGjXhyK62VyJwhPUPLZefKVthcoUcuWK6cqkSU4M542ipNxS3"
                .parse()
                .unwrap()
        }
    }
};

#[cfg(feature = "near-kit")]
const _: () = {
    use near_kit::types::PublicKey as NearPublicKey;

    impl From<NearPublicKey> for PublicKey {
        #[inline]
        fn from(pk: NearPublicKey) -> Self {
            match pk {
                NearPublicKey::Ed25519(pk) => Self::Ed25519(pk.into()),
                NearPublicKey::Secp256k1(pk) => Self::Secp256k1(pk.into()),
            }
        }
    }
};

#[cfg(test)]
mod tests {
    use near_sdk::AccountIdRef;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        "ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJugxm",
        "423df0a6640e9467769c55a573f15b9ee999dc8970048959c72890abf5cc3a8e"
    )]
    #[case(
        "secp256k1:3aMVMxsoAnHUbweXMtdKaN1uJaNwsfKv7wnc97SDGjXhyK62VyJwhPUPLZefKVthcoUcuWK6cqkSU4M542ipNxS3",
        "0xbff77166b39599e54e391156eef7b8191e02be92"
    )]
    #[case(
        "p256:3aMVMxsoAnHUbweXMtdKaN1uJaNwsfKv7wnc97SDGjXhyK62VyJwhPUPLZefKVthcoUcuWK6cqkSU4M542ipNxS3",
        "0x7edf07ede58238026db3f90fc8032633b69b8de5"
    )]
    fn to_implicit_account_id(#[case] pk: &str, #[case] expected: &str) {
        assert_eq!(
            pk.parse::<PublicKey>().unwrap().to_implicit_account_id(),
            AccountIdRef::new_or_panic(expected)
        );
    }

    #[rstest]
    #[case("ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJ")]
    #[case("ed25519:")]
    #[case("secp256k1:p3UPfBR3kWxE2C8wF1855eguaoRvoW6jV5ZXbu3sTTCs")]
    #[case("secp256k1:")]
    #[case("p256:p3UPfBR3kWxE2C8wF1855eguaoRvoW6jV5ZXbu3sTTCs")]
    #[case("p256:")]
    fn parse_invalid_length(#[case] pk: &str) {
        assert_eq!(pk.parse::<PublicKey>(), Err(ParseCurveError::InvalidLength));
    }
}
