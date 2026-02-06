use super::{Curve, CurveType, TypedCurve};
use generic_array::GenericArray;
use near_sdk::CryptoHash;
use p256::{
    ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier},
    elliptic_curve::scalar::IsHigh,
};

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
