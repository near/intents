use core::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use borsh::{BorshDeserialize, BorshSerialize};
use defuse_crypto::{
    ed25519::{Ed25519, Ed25519Signature},
    fmt::{ParseCurveError, TypedCurve, checked_base58_decode_array},
    p256::{P256, P256Signature},
    secp256k1::{Secp256k1, Secp256k1RecoverableSignature},
};
use serde_with::{DeserializeFromStr, SerializeDisplay};

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
pub enum Signature {
    Ed25519(Ed25519Signature) = 0,
    Secp256k1(Secp256k1RecoverableSignature) = 1,
    P256(P256Signature) = 2,
}

impl Debug for Signature {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ed25519(sig) => sig.to_string(),
                Self::Secp256k1(sig) => sig.to_string(),
                Self::P256(sig) => sig.to_string(),
            }
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
        let (curve, data) = s
            .split_once(':')
            // ed25519 by default
            .unwrap_or((Ed25519::CURVE_TYPE, s));

        match curve {
            Ed25519::CURVE_TYPE => checked_base58_decode_array(data)
                .map(Ed25519Signature)
                .map(Into::into),
            Secp256k1::CURVE_TYPE => checked_base58_decode_array(data)
                .map(Secp256k1RecoverableSignature)
                .map(Into::into),
            P256::CURVE_TYPE => checked_base58_decode_array(data)
                .map(P256Signature)
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
                        .map(|s: Self| serde_json::Value::String(s.to_string()))
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
    #[case(
        "ed25519:4nrYPT9gQbagzC1c7gSRnSkjZukXqjFxnPVp6wjmH1QgsBB1xzsbHB3piY7eHBnofUVS4WRRHpSfTVaqYq9KM265"
    )]
    #[case(
        "secp256k1:7o3557Aipc2MDtvh3E5ZQet85ZcRsynThmhcVZye9mUD1fcG6PBCerX6BKDGkKf3L31DUSkAtSd9o4kGvc3h4wZJ7"
    )]
    #[case(
        "p256:4skfJSJRVHKjXs2FztBcSnTsbSRMjF3ykFz9hB4kZo486KvRrTpwz54uzQawsKtCdM1BdQR6JdAAZXmHreNXmNBj"
    )]
    fn parse_ok(#[case] sig: &str) {
        sig.parse::<Signature>().unwrap();
    }

    #[rstest]
    #[case("ed25519:5TagutioHgKLh7KZ1VEFBYfgRkPtqnKm9LoMnJMJ")]
    #[case("ed25519:")]
    #[case("secp256k1:p3UPfBR3kWxE2C8wF1855eguaoRvoW6jV5ZXbu3sTTCs")]
    #[case("secp256k1:")]
    #[case("p256:p3UPfBR3kWxE2C8wF1855eguaoRvoW6jV5ZXbu3sTTCs")]
    #[case("p256:")]
    fn parse_invalid_length(#[case] sig: &str) {
        assert_eq!(
            sig.parse::<Signature>(),
            Err(ParseCurveError::InvalidLength)
        );
    }
}
