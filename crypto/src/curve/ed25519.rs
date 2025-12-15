use ed25519_dalek::VerifyingKey;
use near_sdk::env;

use super::{Curve, CurveType, TypedCurve};

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
