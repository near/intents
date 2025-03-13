use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use near_sdk::{AccountId, AccountIdRef, bs58, env, near};

use crate::{Curve, CurveType, Ed25519, P256, ParseCurveError, Secp256k1};

#[near(serializers = [borsh])]
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr)
)]
pub enum PublicKey {
    Ed25519(<Ed25519 as Curve>::PublicKey),
    Secp256k1(<Secp256k1 as Curve>::PublicKey),
    P256(<P256 as Curve>::PublicKey),
}

impl PublicKey {
    #[inline]
    pub const fn curve_type(&self) -> CurveType {
        match self {
            Self::Ed25519(_) => CurveType::Ed25519,
            Self::Secp256k1(_) => CurveType::Secp256k1,
            Self::P256(_) => CurveType::P256,
        }
    }

    #[inline]
    const fn data(&self) -> &[u8] {
        #[allow(clippy::match_same_arms)]
        match self {
            Self::Ed25519(data) => data,
            Self::Secp256k1(data) => data,
            Self::P256(data) => data,
        }
    }

    #[inline]
    pub fn to_implicit_account_id(&self) -> AccountId {
        match self {
            Self::Ed25519(pk) => {
                // https://docs.near.org/concepts/protocol/account-id#implicit-address
                hex::encode(pk)
            }
            Self::Secp256k1(pk) => {
                // https://ethereum.org/en/developers/docs/accounts/#account-creation
                format!("0x{}", hex::encode(&env::keccak256_array(pk)[12..32]))
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
                    hex::encode(&env::keccak256_array(&[b"p256".as_slice(), pk].concat())[12..32])
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
            (CurveType::Ed25519, s)
        };
        let decoder = bs58::decode(data.as_bytes());
        match curve {
            CurveType::Ed25519 => decoder.into_array_const().map(Self::Ed25519),
            CurveType::Secp256k1 => decoder.into_array_const().map(Self::Secp256k1),
            CurveType::P256 => decoder.into_array_const().map(Self::P256),
        }
        .map_err(Into::into)
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod abi {
    use super::*;

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
                        examples: [Self::example_ed25519(), Self::example_secp256k1()]
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
            "secp256k1:5KN6ZfGZgH1puWwH1Nc1P8xyrFZSPHDw3WUP6iitsjCECJLrGBq"
                .parse()
                .unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implicit_ed25519() {
        assert_eq!(
            "ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJugxm"
                .parse::<PublicKey>()
                .unwrap()
                .to_implicit_account_id(),
            AccountIdRef::new_or_panic(
                "423df0a6640e9467769c55a573f15b9ee999dc8970048959c72890abf5cc3a8e"
            )
        );
    }

    #[test]
    fn implicit_secp256k1() {
        assert_eq!(
            "secp256k1:5KN6ZfGZgH1puWwH1Nc1P8xyrFZSPHDw3WUP6iitsjCECJLrGBq"
                .parse::<PublicKey>()
                .unwrap()
                .to_implicit_account_id(),
            AccountIdRef::new_or_panic("0xbff77166b39599e54e391156eef7b8191e02be92")
        );
    }

    #[test]
    fn implicit_p256() {
        assert_eq!(
            "p256:5KN6ZfGZgH1puWwH1Nc1P8xyrFZSPHDw3WUP6iitsjCECJLrGBq"
                .parse::<PublicKey>()
                .unwrap()
                .to_implicit_account_id(),
            AccountIdRef::new_or_panic("0x7edf07ede58238026db3f90fc8032633b69b8de5")
        );
    }
}
