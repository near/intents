#[cfg(feature = "ed25519")]
pub mod ed25519;

use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use near_sdk::{
    AccountId, bs58, near,
    serde_with::{DeserializeFromStr, SerializeDisplay},
};

use crate::{CurveType, ParseCurveError, parse::checked_base58_decode_array};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh(use_discriminant = true)])]
#[derive(
    Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, SerializeDisplay, DeserializeFromStr,
)]
#[repr(u8)]
pub enum PublicKey {
    #[cfg(feature = "ed25519")]
    Ed25519(<crate::Ed25519 as crate::Curve>::PublicKey) = 0,
    #[cfg(feature = "secp256k1")]
    Secp256k1(<crate::Secp256k1 as crate::Curve>::PublicKey) = 1,
    #[cfg(feature = "p256")]
    P256(crate::P256UncompressedPublicKey) = 2,
}

impl PublicKey {
    #[inline]
    pub const fn curve_type(&self) -> CurveType {
        match self {
            #[cfg(feature = "ed25519")]
            Self::Ed25519(_) => CurveType::Ed25519,
            #[cfg(feature = "secp256k1")]
            Self::Secp256k1(_) => CurveType::Secp256k1,
            #[cfg(feature = "p256")]
            Self::P256(_) => CurveType::P256,
        }
    }

    #[inline]
    const fn data(&self) -> &[u8] {
        #[allow(clippy::match_same_arms)]
        match self {
            #[cfg(feature = "ed25519")]
            Self::Ed25519(data) => data,
            #[cfg(feature = "secp256k1")]
            Self::Secp256k1(data) => data,
            #[cfg(feature = "p256")]
            Self::P256(data) => &data.0,
        }
    }

    #[inline]
    pub fn to_implicit_account_id(&self) -> AccountId {
        match self {
            #[cfg(feature = "ed25519")]
            Self::Ed25519(pk) => {
                // https://docs.near.org/concepts/protocol/account-id#implicit-address
                hex::encode(pk)
            }
            #[cfg(feature = "secp256k1")]
            Self::Secp256k1(pk) => {
                // https://ethereum.org/en/developers/docs/accounts/#account-creation
                format!(
                    "0x{}",
                    hex::encode(&::near_sdk::env::keccak256_array(pk)[12..32])
                )
            }
            #[cfg(feature = "p256")]
            Self::P256(crate::P256UncompressedPublicKey(pk)) => {
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
                        &::near_sdk::env::keccak256_array([b"p256".as_slice(), pk].concat())
                            [12..32]
                    )
                )
            }
        }
        .try_into()
        .unwrap_or_else(|_| unreachable!())
    }

    #[cfg(feature = "ed25519")]
    #[inline]
    pub fn from_implicit_account_id(account_id: &near_sdk::AccountIdRef) -> Option<Self> {
        let mut pk = [0; 32];
        // Only NearImplicitAccount can be reversed
        hex::decode_to_slice(account_id.as_str(), &mut pk).ok()?;
        Some(Self::Ed25519(pk))
    }
}

impl Debug for PublicKey {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}",
            self.curve_type(),
            bs58::encode(self.data()).into_string()
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
        let (curve, data) = if let Some((curve, data)) = s.split_once(':') {
            (
                curve.parse().map_err(|_| ParseCurveError::WrongCurveType)?,
                data,
            )
        } else {
            #[cfg(not(feature = "ed25519"))]
            return Err(ParseCurveError::WrongCurveType);

            #[cfg(feature = "ed25519")]
            (CurveType::Ed25519, s)
        };

        match curve {
            #[cfg(feature = "ed25519")]
            CurveType::Ed25519 => checked_base58_decode_array(data).map(Self::Ed25519),
            #[cfg(feature = "secp256k1")]
            CurveType::Secp256k1 => checked_base58_decode_array(data).map(Self::Secp256k1),
            #[cfg(feature = "p256")]
            CurveType::P256 => checked_base58_decode_array(data)
                .map(crate::P256UncompressedPublicKey)
                .map(Self::P256),
        }
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
const _: () = {
    use near_sdk::{
        schemars::{
            JsonSchema,
            r#gen::SchemaGenerator,
            schema::{InstanceType, Metadata, Schema, SchemaObject},
        },
        serde_json,
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
                extensions: [("contentEncoding", "base58".into())]
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
                metadata: Some(
                    Metadata {
                        examples: [
                            #[cfg(feature = "ed25519")]
                            Self::example_ed25519(),
                            #[cfg(feature = "secp256k1")]
                            Self::example_secp256k1(),
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
        #[cfg(feature = "ed25519")]
        pub(super) fn example_ed25519() -> Self {
            "ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJugxm"
                .parse()
                .unwrap()
        }

        #[cfg(feature = "secp256k1")]
        pub(super) fn example_secp256k1() -> Self {
            "secp256k1:3aMVMxsoAnHUbweXMtdKaN1uJaNwsfKv7wnc97SDGjXhyK62VyJwhPUPLZefKVthcoUcuWK6cqkSU4M542ipNxS3"
                .parse()
                .unwrap()
        }
    }
};

#[cfg(feature = "near-api-types")]
const _: () = {
    use near_api_types::crypto::public_key::{
        ED25519PublicKey, PublicKey as NearPublicKey, Secp256K1PublicKey,
    };

    impl From<NearPublicKey> for PublicKey {
        fn from(pk: NearPublicKey) -> Self {
            match pk {
                #[cfg(feature = "ed25519")]
                NearPublicKey::ED25519(pk) => pk.into(),
                #[cfg(feature = "secp256k1")]
                NearPublicKey::SECP256K1(pk) => pk.into(),
            }
        }
    }

    impl From<ED25519PublicKey> for PublicKey {
        fn from(pk: ED25519PublicKey) -> Self {
            Self::Ed25519(pk.0)
        }
    }

    impl From<Secp256K1PublicKey> for PublicKey {
        fn from(pk: Secp256K1PublicKey) -> Self {
            Self::Secp256k1(pk.0)
        }
    }
};

#[cfg(test)]
mod tests {
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
    fn parse_invalid_length(
        #[values(
            "ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJ",
            "ed25519:",
            "secp256k1:p3UPfBR3kWxE2C8wF1855eguaoRvoW6jV5ZXbu3sTTCs",
            "secp256k1:",
            "p256:p3UPfBR3kWxE2C8wF1855eguaoRvoW6jV5ZXbu3sTTCs",
            "p256:"
        )]
        pk: &str,
    ) {
        assert_eq!(pk.parse::<PublicKey>(), Err(ParseCurveError::InvalidLength));
    }
}
