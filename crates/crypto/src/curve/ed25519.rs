use core::fmt::{self, Debug, Display};
use std::str::FromStr;

use crate::{CurveType, TypedCurve};
use crate::{Curve, ParseCurveError};

pub struct Ed25519;

impl Curve for Ed25519 {
    type PublicKey = [u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
    type Signature = [u8; ed25519_dalek::SIGNATURE_LENGTH];

    type Message = [u8];
    type VerifyingKey = Self::PublicKey;
}

#[cfg(feature = "near-contract")]
impl crate::VerifiableCurve for Ed25519 {
    #[inline]
    fn verify(
        signature: &Self::Signature,
        message: &Self::Message,
        public_key: &Self::VerifyingKey,
    ) -> Option<Self::PublicKey> {
        if ed25519_dalek::VerifyingKey::from_bytes(public_key)
            .ok()?
            .is_weak()
        {
            // prevent using weak (i.e. low order) public keys, see
            // https://github.com/dalek-cryptography/ed25519-dalek#weak-key-forgery-and-verify_strict
            return None;
        }

        near_sdk::env::ed25519_verify(signature, message, public_key)
            .then_some(public_key)
            .copied()
    }
}

impl TypedCurve for Ed25519 {
    const CURVE_TYPE: CurveType = CurveType::Ed25519;
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Ed25519PublicKey(
    #[cfg_attr(feature = "serde", serde_as(as = "crate::serde::AsCurve<Ed25519>"))]
    pub  <Ed25519 as Curve>::PublicKey,
);

impl Debug for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<Ed25519 as TypedCurve>::to_base58(self.0))
    }
}

impl FromStr for Ed25519PublicKey {
    type Err = ParseCurveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ed25519::parse_base58(s).map(Self)
    }
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "borsh",
    derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize),
    cfg_attr(feature = "abi", derive(::borsh::BorshSchema))
)]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Ed25519Signature(
    #[cfg_attr(feature = "serde", serde_as(as = "crate::serde::AsCurve<Ed25519>"))]
    pub  <Ed25519 as Curve>::Signature,
);

impl Debug for Ed25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Ed25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<Ed25519 as TypedCurve>::to_base58(self.0))
    }
}

impl FromStr for Ed25519Signature {
    type Err = ParseCurveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ed25519::parse_base58(s).map(Self)
    }
}
