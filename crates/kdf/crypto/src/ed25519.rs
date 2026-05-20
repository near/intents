use ed25519_dalek::{self, Signature, Verifier, VerifyingKey};

use crate::Curve;

pub struct Ed25519;

impl Curve for Ed25519 {
    type PublicKey = VerifyingKey;

    type Message = [u8];

    type Signature = Signature;

    #[inline]
    fn verify(
        public_key: &Self::PublicKey,
        msg: &Self::Message,
        signature: &Self::Signature,
    ) -> bool {
        public_key.verify(msg, signature).is_ok()
    }
}
