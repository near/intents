use defuse_crypto::Curve;
pub use defuse_crypto::{Ed25519, Ed25519PublicKey, Ed25519Signature};

use crate::signature::SigningStandard;

impl<M> SigningStandard<M> for Ed25519
where
    M: AsRef<[u8]>,
{
    type PublicKey = Ed25519PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(sig) = signature.parse::<Ed25519Signature>() else {
            return false;
        };
        <Self as Curve>::verify(&sig.0, msg.as_ref(), &public_key.0).is_some()
    }
}
