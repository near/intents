use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use near_sdk::{bs58, near};

use crate::{Curve, CurveType, Ed25519, P256, ParseCurveError, Secp256k1};

#[near(serializers = [borsh])]
#[cfg_attr(
    feature = "serde",
    derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr)
)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Signature {
    Ed25519(<Ed25519 as Curve>::Signature),
    Secp256k1(<Secp256k1 as Curve>::Signature),
    P256(<P256 as Curve>::Signature),
}

impl Signature {
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
}

impl Debug for Signature {
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

impl Display for Signature {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for Signature {
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

    impl JsonSchema for Signature {
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

    impl Signature {
        pub(super) fn example_ed25519() -> Self {
            "ed25519:DNxoVu7L7sHr9pcHGWQoJtPsrwheB8akht1JxaGpc9hGrpehdycXBMLJg4ph1bQ9bXdfoxJCbbwxj3Bdrda52eF"
                .parse()
                .unwrap()
        }

        pub(super) fn example_secp256k1() -> Self {
            "secp256k1:7huDZxNnibusy6wFkbUBQ9Rqq2VmCKgTWYdJwcPj8VnciHjZKPa41rn5n6WZnMqSUCGRHWMAsMjKGtMVVmpETCeCs"
                .parse()
                .unwrap()
        }
    }
}
