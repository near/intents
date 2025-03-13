use super::{Curve, CurveType, TypedCurve};
use near_sdk::CryptoHash;
use p256::{
    EncodedPoint,
    ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier},
    elliptic_curve::{generic_array::GenericArray, scalar::IsHigh},
};

pub struct P256;

impl Curve for P256 {
    /// Concatenated `x || y` coordinates with no leading SEC1 tag byte.
    type PublicKey = [u8; 64];

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
        let signature = Signature::from_bytes(GenericArray::from_slice(signature)).ok()?;

        if signature.s().is_high().into() {
            // guard against signature malleability
            return None;
        }

        // convert verifying key
        let verifying_key = VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
            GenericArray::from_slice(public_key),
        ))
        .ok()?;

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
