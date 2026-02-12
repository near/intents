use defuse_crypto::{Curve, Ed25519, Ed25519PublicKey, TypedCurve};

use crate::signature::SigningStandard;

impl<M> SigningStandard<M> for Ed25519
where
    M: AsRef<[u8]>,
{
    type PublicKey = Ed25519PublicKey;

    fn verify(msg: M, public_key: &Self::PublicKey, signature: &str) -> bool {
        let Ok(sig) = Self::parse_base58(signature) else {
            return false;
        };
        <Self as Curve>::verify(&sig, msg.as_ref(), &public_key.0).is_some()
    }
}
