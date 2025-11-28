use schnorrkel::{
    PublicKey as SchnorrkelPublicKey, Signature as SchnorrkelSignature,
};

use super::{Curve, CurveType, TypedCurve};

pub struct Sr25519;

impl Curve for Sr25519 {
    type PublicKey = [u8; 32];
    type Signature = [u8; 64];

    type Message = [u8];
    type VerifyingKey = Self::PublicKey;

    #[inline]
    fn verify(
        signature: &Self::Signature,
        message: &Self::Message,
        public_key: &Self::VerifyingKey,
    ) -> Option<Self::PublicKey> {
        let public_key_parsed = SchnorrkelPublicKey::from_bytes(public_key).ok()?;
        let signature_parsed = SchnorrkelSignature::from_bytes(signature).ok()?;

        // verify_simple in schnorrkel 0.11 signature is:
        // pub fn verify_simple(&self, ctx: &'static [u8], msg: &[u8], sig: &Signature)
        // Using "substrate" as the default context following Substrate/Polkadot convention
        public_key_parsed
            .verify_simple(b"substrate", message, &signature_parsed)
            .ok()?;

        Some(*public_key)
    }
}

impl TypedCurve for Sr25519 {
    const CURVE_TYPE: CurveType = CurveType::Sr25519;
}
