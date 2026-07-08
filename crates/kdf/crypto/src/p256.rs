use p256::ecdsa::{Signature, SigningKey, VerifyingKey, signature::hazmat::PrehashSigner};
pub use p256::*;

use crate::{Curve, Signer};

pub struct P256;

impl Curve for P256 {
    type PublicKey = VerifyingKey;

    type Signature = Signature;

    // TODO: docs: prehash
    #[inline]
    fn verify(public_key: &Self::PublicKey, prehash: &[u8], signature: &Self::Signature) -> bool {
        // accept only 32 byte prehash
        let Ok(prehash) = <&[u8; 32]>::try_from(prehash) else {
            return false;
        };

        cfg_select! {
            // TODO: cfg(near)
            _ => {{
                use p256::{
                    ecdsa::signature::hazmat::PrehashVerifier,
                    elliptic_curve::scalar::IsHigh,
                };

                // TODO: or not?
                // P-256 is the passkey/WebAuthn curve, and WebAuthn does not require low-S — Apple Secure Enclave and various authenticators routinely emit high-S signatures. This is exactly why Ethereum's P256VERIFY precompile (RIP-7212) deliberately does not enforce low-S. So strict low-S rejection here will break signers that emit high-S.
                if signature.s().is_high().into() {
                    // guard against signature malleability
                    return false;
                }

                public_key.verify_prehash(prehash, signature).is_ok()
            }}
        }
    }
}

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
// TODO: docs: untagged uncompressed with no leading SEC-1 tag byte, etc...
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct P256UncompressedPublicKey(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 64],
);

impl P256UncompressedPublicKey {
    /// Compress public key
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_kdf_crypto::p256::{
    /// #     P256CompressedPublicKey,
    /// #     P256UncompressedPublicKey,
    /// # };
    /// # use hex_literal::hex;
    /// assert_eq!(
    ///     P256UncompressedPublicKey(hex!("beed8cb2c3622dd5f1ee641f12d88e35f3fb8c6ae081d689008bdaa6af38d4408e9c469c5ca7b59927606ef9ea34ee2335e85dbeaa265ca038b5e2896f34ded0"))
    ///         .compress().0,
    ///     hex!("02beed8cb2c3622dd5f1ee641f12d88e35f3fb8c6ae081d689008bdaa6af38d440"),
    /// );
    /// ```
    #[inline]
    pub fn compress(&self) -> P256CompressedPublicKey {
        EncodedPoint::from_untagged_bytes(&self.0.into())
            .compress()
            .as_bytes()
            .try_into()
            .map_or_else(
                |_| unreachable!(), // already compressed
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

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
// TODO: docs: compressed with leading SEC-1 tag byte, etc...
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct P256CompressedPublicKey(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 33],
);

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
    #[inline]
    fn from(value: P256UncompressedPublicKey) -> Self {
        (&value).into()
    }
}

impl From<&P256UncompressedPublicKey> for P256CompressedPublicKey {
    #[inline]
    fn from(value: &P256UncompressedPublicKey) -> Self {
        value.compress()
    }
}

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr),
    cfg_attr(feature = "schemars-v0_8", derive(::schemars::JsonSchema))
)]
#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))
)]
// TODO: docs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct P256Signature(
    // schemars@0.8 ignores `with` at struct level for newtypes; must be on the field
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "String"))] pub [u8; 64],
);

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
            f.write_str(&P256::to_base58(&self.0))
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
            f.write_str(&P256::to_base58(&self.0))
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
            f.write_str(&P256::to_base58(&self.0))
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

impl Signer<P256> for SigningKey {
    type Error = Error;

    fn public_key(&self) -> <P256 as Curve>::PublicKey {
        *self.verifying_key()
    }

    fn sign(&self, msg: &[u8]) -> Result<<P256 as Curve>::Signature, Self::Error> {
        self.sign_prehash(msg)
            .map_err(|_| Error::InvalidPrehashLength)
    }
}

pub enum Error {
    InvalidPrehashLength,
}

// TODO: tests
