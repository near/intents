use schnorrkel::{PublicKey, Signature};

use super::{Curve, CurveType, TypedCurve};

pub struct Sr25519;

impl Sr25519 {
    // using "substrate" as the default context following Substrate/Polkadot convention
    const SIGNING_CTX: &[u8] = b"substrate";
}

impl Curve for Sr25519 {
    /// A Ristretto Schnorr public key represented as a 32-byte Ristretto compressed point
    type PublicKey = [u8; schnorrkel::PUBLIC_KEY_LENGTH];
    /// A 64-byte Ristretto Schnorr signature
    type Signature = [u8; schnorrkel::SIGNATURE_LENGTH];

    type Message = [u8];
    type VerifyingKey = Self::PublicKey;

    #[inline]
    fn verify(
        signature: &Self::Signature,
        message: &Self::Message,
        public_key: &Self::VerifyingKey,
    ) -> Option<Self::PublicKey> {
        let public_key_parsed = PublicKey::from_bytes(public_key).ok()?;
        let signature_parsed = Signature::from_bytes(signature).ok()?;

        public_key_parsed
            .verify_simple(Self::SIGNING_CTX, message, &signature_parsed)
            .is_ok()
            .then_some(public_key)
            .copied()
    }
}

impl TypedCurve for Sr25519 {
    const CURVE_TYPE: CurveType = CurveType::Sr25519;
}
