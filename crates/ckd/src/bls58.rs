use std::{fmt, str::FromStr};

use blstrs::{G1Affine, G2Affine};

const G1_PREFIX: &str = "bls12381g1:";
const G2_PREFIX: &str = "bls12381g2:";

#[derive(Debug, thiserror::Error)]
pub enum ParseBlsPointError {
    #[error("wrong or missing {0} prefix")]
    BadPrefix(&'static str),
    #[error("invalid base58 encoding")]
    InvalidBase58,
    #[error("invalid point encoding")]
    InvalidPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid point encoding")]
pub struct InvalidPointError;

fn decode_prefixed<const N: usize>(
    prefix: &'static str,
    s: &str,
) -> Result<[u8; N], ParseBlsPointError> {
    let data = s
        .strip_prefix(prefix)
        .ok_or(ParseBlsPointError::BadPrefix(prefix))?;
    let mut bytes = [0u8; N];
    match bs58::decode(data).onto(&mut bytes) {
        Ok(n) if n == N => Ok(bytes),
        Ok(_) | Err(bs58::decode::Error::BufferTooSmall) => Err(ParseBlsPointError::InvalidPoint),
        Err(_) => Err(ParseBlsPointError::InvalidBase58),
    }
}

#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "schemars-v0_8",
        derive(::schemars::JsonSchema),
        schemars(example = "Self::example")
    )
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::AsRef,
    derive_more::From,
    derive_more::Into,
)]
#[as_ref([u8], [u8; 48])]
#[into(owned, ref)]
#[repr(transparent)]
pub struct Bls12381G1(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 48],
);

impl Bls12381G1 {
    #[cfg(feature = "schemars-v0_8")]
    const fn example() -> Self {
        // compressed encoding of the BLS12-381 G1 generator — a fixed,
        // publicly-known point usable without an RNG.
        Self(hex_literal::hex!(
            "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb"
        ))
    }
}

impl From<G1Affine> for Bls12381G1 {
    fn from(v: G1Affine) -> Self {
        Self(v.to_compressed())
    }
}

impl TryFrom<Bls12381G1> for G1Affine {
    type Error = InvalidPointError;

    fn try_from(v: Bls12381G1) -> Result<Self, Self::Error> {
        Option::from(Self::from_compressed(&v.0)).ok_or(InvalidPointError)
    }
}

impl FromStr for Bls12381G1 {
    type Err = ParseBlsPointError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes: [u8; G1Affine::compressed_size()] = decode_prefixed(G1_PREFIX, s)?;
        Option::<G1Affine>::from(G1Affine::from_compressed(&bytes))
            .ok_or(ParseBlsPointError::InvalidPoint)?;
        Ok(Self(bytes))
    }
}

impl fmt::Display for Bls12381G1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{G1_PREFIX}{}", bs58::encode(self.0).into_string())
    }
}

#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "schemars-v0_8",
        derive(::schemars::JsonSchema),
        schemars(example = "Self::example")
    )
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::AsRef,
    derive_more::From,
    derive_more::Into,
)]
#[as_ref([u8], [u8; 96])]
#[into(owned, ref)]
#[repr(transparent)]
pub struct Bls12381G2(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 96],
);

impl Bls12381G2 {
    #[cfg(feature = "schemars-v0_8")]
    const fn example() -> Self {
        // compressed encoding of the BLS12-381 G2 generator — a fixed,
        // publicly-known point usable without an RNG.
        Self(hex_literal::hex!(
            "93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8"
        ))
    }
}

impl From<G2Affine> for Bls12381G2 {
    fn from(v: G2Affine) -> Self {
        Self(v.to_compressed())
    }
}

impl TryFrom<Bls12381G2> for G2Affine {
    type Error = InvalidPointError;

    fn try_from(v: Bls12381G2) -> Result<Self, Self::Error> {
        Option::from(Self::from_compressed(&v.0)).ok_or(InvalidPointError)
    }
}

impl FromStr for Bls12381G2 {
    type Err = ParseBlsPointError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes: [u8; G2Affine::compressed_size()] = decode_prefixed(G2_PREFIX, s)?;
        Option::<G2Affine>::from(G2Affine::from_compressed(&bytes))
            .ok_or(ParseBlsPointError::InvalidPoint)?;
        Ok(Self(bytes))
    }
}

impl fmt::Display for Bls12381G2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{G2_PREFIX}{}", bs58::encode(self.0).into_string())
    }
}

/// SerDe-friendly mirror of [`crate::AppPublicKeyPV`], convertible via
/// `.into()`/`.try_into()`.
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppPublicKeyPV {
    pub pk1: Bls12381G1,
    pub pk2: Bls12381G2,
}

impl From<crate::AppPublicKeyPV> for AppPublicKeyPV {
    fn from(v: crate::AppPublicKeyPV) -> Self {
        Self {
            pk1: v.pk1.into(),
            pk2: v.pk2.into(),
        }
    }
}

impl TryFrom<AppPublicKeyPV> for crate::AppPublicKeyPV {
    type Error = InvalidPointError;

    /// Only checks that `pk1`/`pk2` are valid on-curve, torsion-free point
    /// encodings — it does **not** imply the result is a valid publicly-verifiable
    /// key (e.g. it does not reject the identity point). Callers must still call
    /// [`crate::AppPublicKeyPV::is_valid`]/[`crate::AppPublicKeyPV::verify`] before
    /// trusting the result, exactly as required when constructing
    /// [`crate::AppPublicKeyPV`] directly from `G1Affine`/`G2Affine`.
    fn try_from(v: AppPublicKeyPV) -> Result<Self, Self::Error> {
        Ok(Self {
            pk1: v.pk1.try_into()?,
            pk2: v.pk2.try_into()?,
        })
    }
}

/// SerDe-friendly mirror of [`crate::CkdResponse`], convertible via
/// `.into()`/`.try_into()`.
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CkdResponse {
    pub big_y: Bls12381G1,
    pub big_c: Bls12381G1,
}

impl From<crate::CkdResponse> for CkdResponse {
    fn from(v: crate::CkdResponse) -> Self {
        Self {
            big_y: v.big_y.into(),
            big_c: v.big_c.into(),
        }
    }
}

impl TryFrom<CkdResponse> for crate::CkdResponse {
    type Error = InvalidPointError;

    /// Only checks that `big_y`/`big_c` are valid on-curve, torsion-free point
    /// encodings — it does **not** imply the result is a valid response. Callers
    /// must still call [`crate::CkdResponse::is_valid`] before trusting the
    /// result, exactly as required when constructing [`crate::CkdResponse`]
    /// directly from `G1Affine`.
    fn try_from(v: CkdResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            big_y: v.big_y.try_into()?,
            big_c: v.big_c.try_into()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    // Base58 encoding of the identity point (compressed: compression + infinity flags
    // set, i.e. byte 0 = 0xc0, all other bytes zero) — a fixed point usable without an RNG.
    const G1_VECTOR: &str =
        "bls12381g1:83VnBdpeT9ioHuUTBD2zcitXg7JqNiebUs5yxDaYaLgZYDLLk2gayHrJKNyRt4vBQB";
    const G2_VECTOR: &str = "bls12381g2:2995BcogDeU9PQxbssLWhG9J76ubezYVpjbceWnwcmRs9ypA2vYDikf8mpuzankQX6WMsDockieKarmDerS2xTPzR1dktiHiGkjvPpRYpDip6unu4WgtPqeCPtxqppMzuGwy";

    #[test]
    fn g1_string_roundtrip() {
        let parsed: Bls12381G1 = G1_VECTOR.parse().unwrap();
        assert_eq!(parsed.to_string(), G1_VECTOR);
    }

    #[test]
    fn g2_string_roundtrip() {
        let parsed: Bls12381G2 = G2_VECTOR.parse().unwrap();
        assert_eq!(parsed.to_string(), G2_VECTOR);
    }

    #[test]
    fn g1_curve_type_roundtrip() {
        let wrapped: Bls12381G1 = G1_VECTOR.parse().unwrap();
        let point: G1Affine = wrapped.try_into().unwrap();
        assert_eq!(Bls12381G1::from(point), wrapped);
    }

    #[test]
    fn g2_curve_type_roundtrip() {
        let wrapped: Bls12381G2 = G2_VECTOR.parse().unwrap();
        let point: G2Affine = wrapped.try_into().unwrap();
        assert_eq!(Bls12381G2::from(point), wrapped);
    }

    #[rstest]
    #[case::wrong_prefix(
        "bls12381g2:83VnBdpeT9ioHuUTBD2zcitXg7JqNiebUs5yxDaYaLgZYDLLk2gayHrJKNyRt4vBQB"
    )]
    #[case::missing_prefix("83VnBdpeT9ioHuUTBD2zcitXg7JqNiebUs5yxDaYaLgZYDLLk2gayHrJKNyRt4vBQB")]
    fn g1_from_str_bad_prefix(#[case] input: &str) {
        assert!(matches!(
            input.parse::<Bls12381G1>(),
            Err(ParseBlsPointError::BadPrefix(G1_PREFIX))
        ));
    }

    #[rstest]
    #[case::not_base58("bls12381g1:not-valid-base58-0OIl")]
    fn g1_from_str_invalid_base58(#[case] input: &str) {
        assert!(matches!(
            input.parse::<Bls12381G1>(),
            Err(ParseBlsPointError::InvalidBase58)
        ));
    }

    #[rstest]
    #[case::wrong_length("bls12381g1:2NEpo7TZRRrLZSi2U")] // valid base58, wrong decoded length
    #[case::not_on_curve(
        "bls12381g1:83VnBdpeT9ioHuUTBD2zcitXg7JqNiebUs5yxDaYaLgZYDLLk2gayHrJKNyRt4vBQC"
    )] // right length, corrupted point
    fn g1_from_str_invalid_point(#[case] input: &str) {
        assert!(matches!(
            input.parse::<Bls12381G1>(),
            Err(ParseBlsPointError::InvalidPoint)
        ));
    }

    #[rstest]
    #[case::wrong_prefix(
        "bls12381g1:2995BcogDeU9PQxbssLWhG9J76ubezYVpjbceWnwcmRs9ypA2vYDikf8mpuzankQX6WMsDockieKarmDerS2xTPzR1dktiHiGkjvPpRYpDip6unu4WgtPqeCPtxqppMzuGwy"
    )]
    #[case::missing_prefix(
        "2995BcogDeU9PQxbssLWhG9J76ubezYVpjbceWnwcmRs9ypA2vYDikf8mpuzankQX6WMsDockieKarmDerS2xTPzR1dktiHiGkjvPpRYpDip6unu4WgtPqeCPtxqppMzuGwy"
    )]
    fn g2_from_str_bad_prefix(#[case] input: &str) {
        assert!(matches!(
            input.parse::<Bls12381G2>(),
            Err(ParseBlsPointError::BadPrefix(G2_PREFIX))
        ));
    }

    #[rstest]
    #[case::not_base58("bls12381g2:not-valid-base58-0OIl")]
    fn g2_from_str_invalid_base58(#[case] input: &str) {
        assert!(matches!(
            input.parse::<Bls12381G2>(),
            Err(ParseBlsPointError::InvalidBase58)
        ));
    }

    #[test]
    fn g2_from_str_wrong_length() {
        // Valid base58, but far too short to be a compressed G2 point.
        assert!(matches!(
            "bls12381g2:2NEpo7TZRRrLZSi2U".parse::<Bls12381G2>(),
            Err(ParseBlsPointError::InvalidPoint)
        ));
    }
}
