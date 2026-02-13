use defuse_crypto::Curve;
pub use defuse_crypto::{Ed25519PublicKey, Ed25519Signature};

use crate::Algorithm;

/// [COSE EdDSA (-8) algorithm](https://www.iana.org/assignments/cose/cose.xhtml#algorithms):
/// ed25519 curve
#[derive(Debug, Clone)]
pub struct Ed25519;

impl Algorithm for Ed25519 {
    type PublicKey = Ed25519PublicKey;
    type Signature = Ed25519Signature;

    #[inline]
    fn verify(msg: &[u8], public_key: &Self::PublicKey, signature: &Self::Signature) -> bool {
        defuse_crypto::Ed25519::verify(&signature.0, msg, &public_key.0).is_some()
    }
}
