use core::fmt::{self, Debug, Display};
use std::str::FromStr;

use generic_array::GenericArray;
use near_sdk::{
    CryptoHash, near,
    serde_with::{DeserializeFromStr, SerializeDisplay},
};
use p256::{
    EncodedPoint,
    ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier},
    elliptic_curve::scalar::IsHigh,
};

use crate::{Curve, CurveType, ParseCurveError, TypedCurve};

pub struct P256;

impl Curve for P256 {
    /// Compressed SEC1 encoded coordinates.
    type PublicKey = [u8; 33];

    /// Concatenated `r || s` coordinates
    type Signature = [u8; 64];

    // Output of cryptographic hash function
    type Message = CryptoHash;

    type VerifyingKey = Self::PublicKey;

    fn verify(
        signature: &Self::Signature,
        prehashed: &Self::Message,
        public_key: &Self::VerifyingKey,
    ) -> Option<Self::PublicKey> {
        // convert signature
        let signature =
            Signature::from_bytes(GenericArray::from_slice(signature).as_0_14()).ok()?;

        if signature.s().is_high().into() {
            // guard against signature malleability
            return None;
        }

        // convert verifying key
        let verifying_key = VerifyingKey::from_sec1_bytes(public_key).ok()?;

        // verify signature over prehashed
        verifying_key
            .verify_prehash(prehashed, &signature)
            .is_ok()
            .then_some(public_key)
            .copied()
    }
}

impl TypedCurve for P256 {
    const CURVE_TYPE: CurveType = CurveType::P256;
}

/// Compressed public key, i.e. `x` coordinate with leading SEC1 tag byte
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh])]
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr,
)]
#[serde_with(crate = "::near_sdk::serde_with")]
#[repr(transparent)]
pub struct P256CompressedPublicKey(pub <P256 as Curve>::PublicKey);

impl Debug for P256CompressedPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for P256CompressedPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<P256 as TypedCurve>::to_base58(self.0))
    }
}

impl FromStr for P256CompressedPublicKey {
    type Err = ParseCurveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        P256::parse_base58(s).map(Self)
    }
}

/// Concatenated `x || y` coordinates with no leading SEC1 tag byte
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh])]
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr,
)]
#[serde_with(crate = "::near_sdk::serde_with")]
#[repr(transparent)]
pub struct P256UncompressedPublicKey(pub [u8; 64]);

impl Debug for P256UncompressedPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for P256UncompressedPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<P256 as TypedCurve>::to_base58(self.0))
    }
}

impl FromStr for P256UncompressedPublicKey {
    type Err = ParseCurveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        P256::parse_base58(s).map(Self)
    }
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh])]
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr,
)]
#[serde_with(crate = "::near_sdk::serde_with")]
#[repr(transparent)]
pub struct P256Signature(pub <P256 as Curve>::Signature);

impl Debug for P256Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for P256Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<P256 as TypedCurve>::to_base58(self.0))
    }
}

impl FromStr for P256Signature {
    type Err = ParseCurveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        P256::parse_base58(s).map(Self)
    }
}

/// Converts from untagged uncompressed form (i.e. concatenated `x || y`
/// coordinates with no leading SEC1 tag byte) into compressed form
/// (i.e. `x` coordinate with leading SEC1 tag byte)
pub fn compress_public_key(public_key: P256UncompressedPublicKey) -> P256CompressedPublicKey {
    EncodedPoint::from_untagged_bytes(GenericArray::from_array(public_key.0).as_0_14())
        .compress()
        .as_bytes()
        .try_into()
        .map_or_else(|_| unreachable!(), P256CompressedPublicKey)
}
