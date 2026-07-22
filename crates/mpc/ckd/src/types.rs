use std::{fmt, str::FromStr};

use blstrs::{G1Affine, G2Affine};

/// Compressed G1 point of BLS12-381 curve
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "schemars-v0_8",
        derive(::schemars::JsonSchema),
        schemars(example = "Self::example")
    )
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
pub struct Bls12381G1Compressed(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 48],
);

impl Bls12381G1Compressed {
    #[cfg(feature = "schemars-v0_8")]
    fn example() -> Self {
        use pairing::group::prime::PrimeCurveAffine;
        G1Affine::generator().into()
    }
}

impl From<G1Affine> for Bls12381G1Compressed {
    #[inline]
    fn from(v: G1Affine) -> Self {
        (&v).into()
    }
}

impl From<&G1Affine> for Bls12381G1Compressed {
    #[inline]
    fn from(v: &G1Affine) -> Self {
        Self(v.to_compressed())
    }
}

impl TryFrom<Bls12381G1Compressed> for G1Affine {
    type Error = InvalidPointError;

    #[inline]
    fn try_from(v: Bls12381G1Compressed) -> Result<Self, Self::Error> {
        (&v).try_into()
    }
}

impl TryFrom<&Bls12381G1Compressed> for G1Affine {
    type Error = InvalidPointError;

    #[inline]
    fn try_from(v: &Bls12381G1Compressed) -> Result<Self, Self::Error> {
        // TODO: cfg(near): bls12381_p1_decompress
        Self::from_compressed(&v.0)
            .into_option()
            .ok_or(InvalidPointError)
    }
}

/// Compressed G2 point of BLS12-381 curve
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "schemars-v0_8",
        derive(::schemars::JsonSchema),
        schemars(example = "Self::example")
    )
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
pub struct Bls12381G2Compressed(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 96],
);

impl Bls12381G2Compressed {
    #[cfg(feature = "schemars-v0_8")]
    fn example() -> Self {
        use pairing::group::prime::PrimeCurveAffine;
        G2Affine::generator().into()
    }
}

impl From<G2Affine> for Bls12381G2Compressed {
    #[inline]
    fn from(v: G2Affine) -> Self {
        (&v).into()
    }
}

impl From<&G2Affine> for Bls12381G2Compressed {
    #[inline]
    fn from(v: &G2Affine) -> Self {
        Self(v.to_compressed())
    }
}

impl TryFrom<Bls12381G2Compressed> for G2Affine {
    type Error = InvalidPointError;

    #[inline]
    fn try_from(v: Bls12381G2Compressed) -> Result<Self, Self::Error> {
        (&v).try_into()
    }
}

impl TryFrom<&Bls12381G2Compressed> for G2Affine {
    type Error = InvalidPointError;

    #[inline]
    fn try_from(v: &Bls12381G2Compressed) -> Result<Self, Self::Error> {
        // TODO: cfg(near): bls12381_p2_decompress
        Self::from_compressed(&v.0)
            .into_option()
            .ok_or(InvalidPointError)
    }
}

#[cfg(feature = "fmt")]
const _: () = {
    use defuse_crypto::fmt::{ParseCurveError, TypedCurve};

    struct Bls12381G1;
    impl TypedCurve for Bls12381G1 {
        const CURVE_TYPE: &str = "bls12381g1";
    }

    struct Bls12381G2;
    impl TypedCurve for Bls12381G2 {
        const CURVE_TYPE: &str = "bls12381g2";
    }

    impl fmt::Display for Bls12381G1Compressed {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&Bls12381G1::to_base58(self.0))
        }
    }

    impl FromStr for Bls12381G1Compressed {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Bls12381G1::parse_base58(s).map(Self)
        }
    }

    impl fmt::Display for Bls12381G2Compressed {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&Bls12381G2::to_base58(self.0))
        }
    }

    impl FromStr for Bls12381G2Compressed {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Bls12381G2::parse_base58(s).map(Self)
        }
    }
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid point encoding")]
pub struct InvalidPointError;

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
        let parsed: Bls12381G1Compressed = G1_VECTOR.parse().unwrap();
        assert_eq!(parsed.to_string(), G1_VECTOR);
    }

    #[test]
    fn g2_string_roundtrip() {
        let parsed: Bls12381G2Compressed = G2_VECTOR.parse().unwrap();
        assert_eq!(parsed.to_string(), G2_VECTOR);
    }

    #[test]
    fn g1_curve_type_roundtrip() {
        let wrapped: Bls12381G1Compressed = G1_VECTOR.parse().unwrap();
        let point: G1Affine = wrapped.try_into().unwrap();
        assert_eq!(Bls12381G1Compressed::from(point), wrapped);
    }

    #[test]
    fn g2_curve_type_roundtrip() {
        let wrapped: Bls12381G2Compressed = G2_VECTOR.parse().unwrap();
        let point: G2Affine = wrapped.try_into().unwrap();
        assert_eq!(Bls12381G2Compressed::from(point), wrapped);
    }

    #[rstest]
    #[case::wrong_prefix(
        "bls12381g2:83VnBdpeT9ioHuUTBD2zcitXg7JqNiebUs5yxDaYaLgZYDLLk2gayHrJKNyRt4vBQB"
    )]
    #[case::not_base58("bls12381g1:not-valid-base58-0OIl")]
    #[case::wrong_length("bls12381g1:2NEpo7TZRRrLZSi2U")] // valid base58, wrong decoded length
    fn g1_parse_err(#[case] input: &str) {
        input.parse::<Bls12381G1Compressed>().unwrap_err();
    }

    #[rstest]
    #[case::wrong_prefix(
        "bls12381g1:2995BcogDeU9PQxbssLWhG9J76ubezYVpjbceWnwcmRs9ypA2vYDikf8mpuzankQX6WMsDockieKarmDerS2xTPzR1dktiHiGkjvPpRYpDip6unu4WgtPqeCPtxqppMzuGwy"
    )]
    #[case::not_base58("bls12381g2:not-valid-base58-0OIl")]
    #[case::wrong_length("bls12381g2:2NEpo7TZRRrLZSi2U")] // valid base58, wrong decoded length
    fn g2_parse_err(#[case] input: &str) {
        input.parse::<Bls12381G2Compressed>().unwrap_err();
    }
}
