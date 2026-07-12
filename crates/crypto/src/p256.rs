pub use p256;
use p256::{
    EncodedPoint,
    ecdsa::{Signature, VerifyingKey},
    elliptic_curve::scalar::IsHigh,
};

use crate::Curve;

/// P256 (a.k.a. secp256r1) Elliptic Curve Digital Signature Algorithm
pub struct P256;

impl Curve for P256 {
    type PublicKey = VerifyingKey;

    type Signature = Signature;

    /// Verify P256 signature over **32-byte prehash** (i.e. output of
    /// cryptographic hash function) for given public key.
    #[inline]
    fn verify(public_key: &Self::PublicKey, prehash: &[u8], signature: &Self::Signature) -> bool {
        // accept only 32 byte prehash
        let Ok(prehash) = <&[u8; 32]>::try_from(prehash) else {
            return false;
        };

        if signature.s().is_high().into() {
            // guard against signature malleability
            return false;
        }

        cfg_select! {
            // TODO: cfg(near)
            _ => {{
                use p256::ecdsa::signature::hazmat::PrehashVerifier;

                public_key.verify_prehash(prehash, signature).is_ok()
            }}
        }
    }
}

/// Uncompressed P256 public key **without** leading SEC-1 tag byte.
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "schemars-v0_8",
        derive(::schemars::JsonSchema),
        schemars(example = "Self::example"),
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
#[as_ref([u8], [u8; 64])]
#[into(owned, ref)]
#[repr(transparent)]
pub struct P256UncompressedPublicKey(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 64],
);

impl P256UncompressedPublicKey {
    #[cfg(feature = "schemars-v0_8")]
    const fn example() -> Self {
        Self(hex_literal::hex!(
            "b87da10683d04e6ec4e2f1775556a63cbb01be843058eb737fe02f9e22663093cf61b416dfb33ba2e32e9c6f3a09d84fa90d0ab9709f4a298d52e0799c8217f5"
        ))
    }
}

impl P256UncompressedPublicKey {
    /// Compress public key
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_crypto::p256::{
    /// #     P256CompressedPublicKey,
    /// #     P256UncompressedPublicKey,
    /// # };
    /// # use hex_literal::hex;
    /// assert_eq!(
    ///     P256UncompressedPublicKey(hex!("beed8cb2c3622dd5f1ee641f12d88e35f3fb8c6ae081d689008bdaa6af38d4408e9c469c5ca7b59927606ef9ea34ee2335e85dbeaa265ca038b5e2896f34ded0"))
    ///         .compress()
    ///         .0,
    ///     hex!("02beed8cb2c3622dd5f1ee641f12d88e35f3fb8c6ae081d689008bdaa6af38d440"),
    /// );
    /// ```
    #[inline]
    pub fn compress(&self) -> P256CompressedPublicKey {
        EncodedPoint::from_untagged_bytes((&self.0).into())
            .compress()
            .as_bytes()
            .try_into()
            .map_or_else(
                |_| unreachable!(), // compressed key is exactly 33 bytes
                P256CompressedPublicKey,
            )
    }
}

impl From<VerifyingKey> for P256UncompressedPublicKey {
    #[inline]
    fn from(value: VerifyingKey) -> Self {
        (&value).into()
    }
}

impl From<&VerifyingKey> for P256UncompressedPublicKey {
    #[inline]
    fn from(value: &VerifyingKey) -> Self {
        Self(
            value
                .to_encoded_point(false) // do not compress
                .as_bytes()[1..] // skip SEC-1 leading tag byte
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
        )
    }
}

impl TryFrom<P256UncompressedPublicKey> for VerifyingKey {
    type Error = p256::ecdsa::Error;

    #[inline]
    fn try_from(value: P256UncompressedPublicKey) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&P256UncompressedPublicKey> for VerifyingKey {
    type Error = p256::ecdsa::Error;

    #[inline]
    fn try_from(value: &P256UncompressedPublicKey) -> Result<Self, Self::Error> {
        Self::from_encoded_point(&EncodedPoint::from_untagged_bytes((&value.0).into()))
    }
}

/// Compressed P256 public key, i.e. `x` coordinate **with** leading SEC-1 tag byte.
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "schemars-v0_8",
        derive(::schemars::JsonSchema),
        schemars(example = "Self::example"),
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
#[as_ref([u8], [u8; 33])]
#[into(owned, ref)]
#[repr(transparent)]
pub struct P256CompressedPublicKey(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 33],
);

impl P256CompressedPublicKey {
    #[cfg(feature = "schemars-v0_8")]
    const fn example() -> Self {
        Self(hex_literal::hex!(
            "03b87da10683d04e6ec4e2f1775556a63cbb01be843058eb737fe02f9e22663093"
        ))
    }
}

impl From<VerifyingKey> for P256CompressedPublicKey {
    #[inline]
    fn from(value: VerifyingKey) -> Self {
        (&value).into()
    }
}

impl From<&VerifyingKey> for P256CompressedPublicKey {
    #[inline]
    fn from(value: &VerifyingKey) -> Self {
        Self(
            value
                .to_encoded_point(true) // compress
                .as_bytes()
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
        )
    }
}

impl TryFrom<P256CompressedPublicKey> for VerifyingKey {
    type Error = p256::ecdsa::Error;

    #[inline]
    fn try_from(value: P256CompressedPublicKey) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&P256CompressedPublicKey> for VerifyingKey {
    type Error = p256::ecdsa::Error;

    #[inline]
    fn try_from(value: &P256CompressedPublicKey) -> Result<Self, Self::Error> {
        Self::from_sec1_bytes(&value.0)
    }
}

impl From<P256UncompressedPublicKey> for P256CompressedPublicKey {
    /// Compress a public key
    #[inline]
    fn from(value: P256UncompressedPublicKey) -> Self {
        (&value).into()
    }
}

impl From<&P256UncompressedPublicKey> for P256CompressedPublicKey {
    /// Compress a public key
    #[inline]
    fn from(value: &P256UncompressedPublicKey) -> Self {
        value.compress()
    }
}

/// P256 signature
#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(
        feature = "schemars-v0_8",
        derive(::schemars::JsonSchema),
        schemars(example = "Self::example"),
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
#[as_ref([u8], [u8; 64])]
#[into(owned, ref)]
#[repr(transparent)]
pub struct P256Signature(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 64],
);

impl P256Signature {
    #[cfg(feature = "schemars-v0_8")]
    const fn example() -> Self {
        Self(hex_literal::hex!(
            "002527c17a17d709ab62ffab033e0a26d901f5bee6686a0d605032828e490a7c47a9f4161e6a17688ae42a66adb67c9c22c86153afb08103b1e9eccd5314c271"
        ))
    }
}

impl From<Signature> for P256Signature {
    #[inline]
    fn from(value: Signature) -> Self {
        (&value).into()
    }
}

impl From<&Signature> for P256Signature {
    #[inline]
    fn from(value: &Signature) -> Self {
        Self(value.to_bytes().into())
    }
}

impl TryFrom<P256Signature> for Signature {
    type Error = p256::ecdsa::Error;

    #[inline]
    fn try_from(value: P256Signature) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&P256Signature> for Signature {
    type Error = p256::ecdsa::Error;

    #[inline]
    fn try_from(value: &P256Signature) -> Result<Self, Self::Error> {
        Self::from_bytes((&value.0).into())
    }
}

#[cfg(feature = "fmt")]
const _: () = {
    use core::{
        fmt::{self, Display},
        str::FromStr,
    };

    use crate::fmt::{ParseCurveError, TypedCurve};

    impl TypedCurve for P256 {
        const CURVE_TYPE: &str = "p256";
    }

    impl Display for P256UncompressedPublicKey {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&P256::to_base58(self.0))
        }
    }

    impl FromStr for P256UncompressedPublicKey {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            P256::parse_base58(s).map(Self)
        }
    }

    impl Display for P256CompressedPublicKey {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&P256::to_base58(self.0))
        }
    }

    impl FromStr for P256CompressedPublicKey {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            P256::parse_base58(s).map(Self)
        }
    }

    impl Display for P256Signature {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&P256::to_base58(self.0))
        }
    }

    impl FromStr for P256Signature {
        type Err = ParseCurveError;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            P256::parse_base58(s).map(Self)
        }
    }
};

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        hex!("03b87da10683d04e6ec4e2f1775556a63cbb01be843058eb737fe02f9e22663093"),
        hex!("0fc6357f688865a5a5ce1ac484f8ade578325ccda012091a533924404890e794"),
        hex!("002527c17a17d709ab62ffab033e0a26d901f5bee6686a0d605032828e490a7c47a9f4161e6a17688ae42a66adb67c9c22c86153afb08103b1e9eccd5314c271"),
    )]
    #[case(
        hex!("03f616f10f0841ff81b0caa52859ee168d03b657c6adef468761289fe75b8e291c"),
        hex!("b64fd2202ddf8ef361dcbdf224fe6c2a06bd1fc67f40a89cb49a92d2071ffecc"),
        hex!("6bb2de380eefd199ee44364902f4b89679b8df04024e24047dc0ec1c61c11b2d527495fed4a6903ebe086e831bd000bf683c2a36f0454eaccaecf109678fd33e"),
    )]
    fn verify_ok(
        #[case] public_key: impl Into<P256CompressedPublicKey>,
        #[case] prehash: [u8; 32],
        #[case] signature: impl Into<P256Signature>,
    ) {
        assert!(
            P256::verify(
                &public_key.into().try_into().unwrap(),
                &prehash,
                &signature.into().try_into().unwrap(),
            ),
            "signature is invalid",
        );
    }

    #[rstest]
    #[case(
        hex!("03b87da10683d04e6ec4e2f1775556a63cbb01be843058eb737fe02f9e22663093"),
        hex!("0fc6357f688865a5a5ce1ac484f8ade578325ccda012091a533924404890e794"),
        hex!("6bb2de380eefd199ee44364902f4b89679b8df04024e24047dc0ec1c61c11b2d527495fed4a6903ebe086e831bd000bf683c2a36f0454eaccaecf109678fd33e"),
    )]
    #[case(
        hex!("03f616f10f0841ff81b0caa52859ee168d03b657c6adef468761289fe75b8e291c"),
        hex!("b64fd2202ddf8ef361dcbdf224fe6c2a06bd1fc67f40a89cb49a92d2071ffecc"),
        hex!("002527c17a17d709ab62ffab033e0a26d901f5bee6686a0d605032828e490a7c47a9f4161e6a17688ae42a66adb67c9c22c86153afb08103b1e9eccd5314c271"),
    )]
    #[case(
        hex!("03b87da10683d04e6ec4e2f1775556a63cbb01be843058eb737fe02f9e22663093"),
        hex!("b64fd2202ddf8ef361dcbdf224fe6c2a06bd1fc67f40a89cb49a92d2071ffecc"),
        hex!("6bb2de380eefd199ee44364902f4b89679b8df04024e24047dc0ec1c61c11b2d527495fed4a6903ebe086e831bd000bf683c2a36f0454eaccaecf109678fd33e"),
    )]
    fn verify_fail(
        #[case] public_key: impl Into<P256CompressedPublicKey>,
        #[case] prehash: [u8; 32],
        #[case] signature: impl Into<P256Signature>,
    ) {
        assert!(
            !P256::verify(
                &public_key.into().try_into().unwrap(),
                &prehash,
                &signature.into().try_into().unwrap(),
            ),
            "invalid signature passed verification",
        );
    }
}
