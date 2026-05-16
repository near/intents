use k256::ecdsa::signature::hazmat::PrehashVerifier;
pub use k256::{
    self,
    ecdsa::{RecoveryId, Signature, VerifyingKey},
};

use crate::Curve;

pub struct Secp256k1;

impl Curve for Secp256k1 {
    type PublicKey = VerifyingKey;
    /// Prehash, i.e. output of a cryptographic hash function
    type Message = [u8; 32];
    type Signature = (Signature, RecoveryId);

    fn verify(
        public_key: &VerifyingKey,
        prehash: &[u8; 32],
        (signature, _recovery_id): &Self::Signature,
    ) -> bool {
        public_key.verify_prehash(prehash, signature).is_ok()
    }
}
