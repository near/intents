#![cfg(any(feature = "ed25519", feature = "secp256k1", feature = "p256"))]

use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use near_sdk::{
    bs58, near,
    serde_with::{DeserializeFromStr, SerializeDisplay},
};

#[cfg(feature = "ed25519")]
use crate::Ed25519;
#[cfg(feature = "p256")]
use crate::P256;
#[cfg(feature = "secp256k1")]
use crate::Secp256k1;

use crate::{Curve, CurveType, ParseCurveError, parse::checked_base58_decode_array};

#[near(serializers = [borsh(use_discriminant = true)])]
#[derive(
    Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, SerializeDisplay, DeserializeFromStr,
)]
#[repr(u8)]
pub enum Signature {
    #[cfg(feature = "ed25519")]
    Ed25519(<Ed25519 as Curve>::Signature) = 0,
    #[cfg(feature = "secp256k1")]
    Secp256k1(<Secp256k1 as Curve>::Signature) = 1,
    #[cfg(feature = "p256")]
    P256(<P256 as Curve>::Signature) = 2,
}

impl Signature {
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
            CurveType::P256 => checked_base58_decode_array(data).map(Self::P256),
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
                        examples: [
                            #[cfg(feature = "ed25519")]
                            Self::example_ed25519(),
                            #[cfg(feature = "secp256k1")]
                            Self::example_secp256k1(),
                            #[cfg(feature = "p256")]
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

    impl Signature {
        #[cfg(feature = "ed25519")]
        pub(super) fn example_ed25519() -> Self {
            "ed25519:DNxoVu7L7sHr9pcHGWQoJtPsrwheB8akht1JxaGpc9hGrpehdycXBMLJg4ph1bQ9bXdfoxJCbbwxj3Bdrda52eF"
                .parse()
                .unwrap()
        }

        #[cfg(feature = "secp256k1")]
        pub(super) fn example_secp256k1() -> Self {
            "secp256k1:7huDZxNnibusy6wFkbUBQ9Rqq2VmCKgTWYdJwcPj8VnciHjZKPa41rn5n6WZnMqSUCGRHWMAsMjKGtMVVmpETCeCs"
                .parse()
                .unwrap()
        }

        #[cfg(feature = "p256")]
        pub(super) fn example_p256() -> Self {
            "p256:DNxoVu7L7sHr9pcHGWQoJtPsrwheB8akht1JxaGpc9hGrpehdycXBMLJg4ph1bQ9bXdfoxJCbbwxj3Bdrda52eF"
                .parse()
                .unwrap()
        }
    }
};

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[cfg_attr(
        feature = "ed25519",
        case(
            "ed25519:4nrYPT9gQbagzC1c7gSRnSkjZukXqjFxnPVp6wjmH1QgsBB1xzsbHB3piY7eHBnofUVS4WRRHpSfTVaqYq9KM265",
        )
    )]
    #[cfg_attr(
        feature = "secp256k1",
        case(
            "secp256k1:7o3557Aipc2MDtvh3E5ZQet85ZcRsynThmhcVZye9mUD1fcG6PBCerX6BKDGkKf3L31DUSkAtSd9o4kGvc3h4wZJ7",
        )
    )]
    #[cfg_attr(
        feature = "p256",
        case(
            "p256:4skfJSJRVHKjXs2FztBcSnTsbSRMjF3ykFz9hB4kZo486KvRrTpwz54uzQawsKtCdM1BdQR6JdAAZXmHreNXmNBj",
        )
    )]
    fn parse_ok(#[case] sig: &str) {
        sig.parse::<Signature>().unwrap();
    }

    #[rstest]
    #[cfg_attr(
        feature = "ed25519",
        case("ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJ"),
        case("ed25519:")
    )]
    #[cfg_attr(
        feature = "secp256k1",
        case("secp256k1:p3UPfBR3kWxE2C8wF1855eguaoRvoW6jV5ZXbu3sTTCs"),
        case("secp256k1:")
    )]
    #[cfg_attr(
        feature = "p256",
        case("p256:p3UPfBR3kWxE2C8wF1855eguaoRvoW6jV5ZXbu3sTTCs"),
        case("p256:")
    )]
    fn parse_invalid_length(#[case] sig: &str) {
        assert_eq!(
            sig.parse::<Signature>(),
            Err(ParseCurveError::InvalidLength)
        );
    }
}
