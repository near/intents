pub use p256::*;
use p256::{
    ecdsa::{Signature, SigningKey, VerifyingKey, signature::hazmat::PrehashSigner},
    elliptic_curve::scalar::IsHigh,
};

use crate::{Curve, Signer};

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
/// Uncompressed P256 public key **without** leading SEC-1 tag byte.
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
/// Compressed P256 public key, i.e. `x` coordinate **with** leading SEC-1 tag byte.
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
/// P256 signature
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

// TODO: fix test values
// #[cfg(test)]
// mod tests {
//     use hex_literal::hex;
//     use rstest::rstest;

//     use super::*;

//     #[rstest]
//     #[case(
//         hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
//         hex!("aa05af77f274774b8bdc7b61d98bc40da523dc2821fdea555f4d6aa413199bcc"),
//         hex!("7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e47"),
//     )]
//     #[case(
//         hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
//         hex!("1632c0ebba467e157675403ba3ba280b836e1801b5678d878dfc90bfc403d6e1"),
//         hex!("eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c"),
//     )]
//     fn verify_ok(
//         #[case] public_key: [u8; 64],
//         #[case] prehash: [u8; 32],
//         #[case] signature: [u8; 64],
//     ) {
//         assert!(
//             P256::verify(
//                 &P256UncompressedPublicKey(public_key).try_into().unwrap(),
//                 &prehash,
//                 &P256Signature(signature).try_into().unwrap(),
//             ),
//             "signature is invalid",
//         );
//     }

//     #[rstest]
//     #[case(
//         hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
//         hex!("1632c0ebba467e157675403ba3ba280b836e1801b5678d878dfc90bfc403d6e1"),
//         hex!("7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e47"),
//     )]
//     #[case(
//         hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
//         hex!("aa05af77f274774b8bdc7b61d98bc40da523dc2821fdea555f4d6aa413199bcc"),
//         hex!("eea1651a60600ec4d9c45e8ae81da1a78377f789f0ac2019de66ad943459913015ef9256809ee0e6bb76e303a0b4802e475c1d26ade5d585292b80c9fe9cb10c"),
//     )]
//     fn verify_fail(
//         #[case] public_key: [u8; 64],
//         #[case] prehash: [u8; 32],
//         #[case] signature: [u8; 64],
//     ) {
//         assert!(
//             !P256::verify(
//                 &P256UncompressedPublicKey(public_key).try_into().unwrap(),
//                 &prehash,
//                 &P256Signature(signature).try_into().unwrap(),
//             ),
//             "invalid signature passed verification",
//         );
//     }
// }
