#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    borsh(use_discriminant = true),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
/// Admin's public key
pub enum AdminPublicKey {
    Ed25519(defuse_crypto::ed25519::Ed25519PublicKey) = 0,
}

#[cfg(feature = "parse")]
const _: () = {
    use std::{fmt, str::FromStr};

    use defuse_crypto::{
        ed25519::{Ed25519, Ed25519PublicKey},
        fmt::{ParseCurveError, TypedCurve},
    };

    impl fmt::Debug for AdminPublicKey {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }

    impl fmt::Display for AdminPublicKey {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Ed25519(pk) => fmt::Display::fmt(pk, f),
            }
        }
    }

    impl FromStr for AdminPublicKey {
        type Err = ParseCurveError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let (curve, _) = s.split_once(':').ok_or(ParseCurveError::WrongCurveType)?;
            if curve.eq_ignore_ascii_case(Ed25519::CURVE_TYPE) {
                Ed25519::parse_base58(s)
                    .map(Ed25519PublicKey)
                    .map(Self::Ed25519)
            } else {
                Err(ParseCurveError::WrongCurveType)
            }
        }
    }
};
