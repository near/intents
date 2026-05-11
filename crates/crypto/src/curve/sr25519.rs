use core::fmt::{self, Debug, Display};
use std::str::FromStr;

use near_sdk::{
    near,
    serde_with::{DeserializeFromStr, SerializeDisplay},
};
use schnorrkel::{PublicKey, Signature};

use crate::{Curve, CurveType, ParseCurveError, TypedCurve};

pub struct Sr25519;

impl Sr25519 {
    /// Default Substrate/Polkadot signing context used by `sign_simple` /
    /// `verify_simple` in `schnorrkel`. Matches what Polkadot.js Extension,
    /// Talisman, Subwallet, etc. use when signing arbitrary messages.
    pub const SIGNING_CTX: &'static [u8] = b"substrate";
}

impl Curve for Sr25519 {
    /// Ristretto Schnorr public key (32-byte compressed Ristretto point).
    type PublicKey = [u8; schnorrkel::PUBLIC_KEY_LENGTH];
    /// 64-byte Ristretto Schnorr signature.
    type Signature = [u8; schnorrkel::SIGNATURE_LENGTH];

    type Message = [u8];
    type VerifyingKey = Self::PublicKey;

    #[inline]
    fn verify(
        signature: &Self::Signature,
        message: &Self::Message,
        public_key: &Self::VerifyingKey,
    ) -> Option<Self::PublicKey> {
        let pk = PublicKey::from_bytes(public_key).ok()?;
        let sig = Signature::from_bytes(signature).ok()?;
        pk.verify_simple(Self::SIGNING_CTX, message, &sig)
            .is_ok()
            .then_some(public_key)
            .copied()
    }
}

impl TypedCurve for Sr25519 {
    const CURVE_TYPE: CurveType = CurveType::Sr25519;
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh])]
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr,
)]
#[serde_with(crate = "::near_sdk::serde_with")]
#[repr(transparent)]
pub struct Sr25519PublicKey(pub <Sr25519 as Curve>::PublicKey);

impl Debug for Sr25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Sr25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<Sr25519 as TypedCurve>::to_base58(self.0))
    }
}

impl FromStr for Sr25519PublicKey {
    type Err = ParseCurveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Sr25519::parse_base58(s).map(Self)
    }
}

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh])]
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDisplay, DeserializeFromStr,
)]
#[serde_with(crate = "::near_sdk::serde_with")]
#[repr(transparent)]
pub struct Sr25519Signature(pub <Sr25519 as Curve>::Signature);

impl Debug for Sr25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Sr25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&<Sr25519 as TypedCurve>::to_base58(self.0))
    }
}

impl FromStr for Sr25519Signature {
    type Err = ParseCurveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Sr25519::parse_base58(s).map(Self)
    }
}
