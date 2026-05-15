pub use ed25519_dalek::{self, Signature, VerifyingKey};

use crate::Curve;

pub struct Ed25519;

impl Curve for Ed25519 {
    type PublicKey = VerifyingKey;

    type Message = [u8];

    type Signature = Signature;

    fn verify(
        public_key: &Self::PublicKey,
        msg: &Self::Message,
        signature: &Self::Signature,
    ) -> bool {
        // TODO: no strict?
        public_key.verify_strict(msg, signature).is_ok()
    }
}
