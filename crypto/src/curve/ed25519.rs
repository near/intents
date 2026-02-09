use core::fmt::{self, Debug, Display};

use ed25519_dalek::VerifyingKey;
use near_sdk::{env, near};

use crate::{Curve, CurveType, TypedCurve, serde::AsCurve};

pub struct Ed25519;

impl Curve for Ed25519 {
    type PublicKey = [u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
    type Signature = [u8; ed25519_dalek::SIGNATURE_LENGTH];

    type Message = [u8];
    type VerifyingKey = Self::PublicKey;

    #[inline]
    fn verify(
        signature: &Self::Signature,
        message: &Self::Message,
        public_key: &Self::VerifyingKey,
    ) -> Option<Self::PublicKey> {
        if VerifyingKey::from_bytes(public_key).ok()?.is_weak() {
            // prevent using weak (i.e. low order) public keys, see
            // https://github.com/dalek-cryptography/ed25519-dalek#weak-key-forgery-and-verify_strict
            return None;
        }

        env::ed25519_verify(signature, message, public_key)
            .then_some(public_key)
            .copied()
    }
}

impl TypedCurve for Ed25519 {
    const CURVE_TYPE: CurveType = CurveType::Ed25519;
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Ed25519PublicKey(#[serde_as(as = "AsCurve<Ed25519>")] pub <Ed25519 as Curve>::PublicKey);

impl Debug for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<Ed25519 as TypedCurve>::to_base58(&self.0))
    }
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Ed25519Signature(#[serde_as(as = "AsCurve<Ed25519>")] pub <Ed25519 as Curve>::Signature);

impl Debug for Ed25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Ed25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<Ed25519 as TypedCurve>::to_base58(&self.0))
    }
}
