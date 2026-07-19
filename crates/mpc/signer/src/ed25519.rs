use defuse_kdf::crypto::ed25519::{Ed25519, ed25519_dalek::Signature};

use crate::{
    OnChainNearMpcCurve,
    contract::{Payload, PublicKey, SignResponse},
};

impl OnChainNearMpcCurve for Ed25519 {
    #[inline]
    fn extract_public_key(public_key: PublicKey) -> Option<Self::PublicKey> {
        let PublicKey::Ed25519(pk) = public_key else {
            return None;
        };
        pk.try_into().ok()
    }

    #[inline]
    fn msg_to_payload(msg: &[u8]) -> Option<Payload<'_>> {
        Some(Payload::Eddsa(msg.into()))
    }

    #[inline]
    fn extract_signature(sig: SignResponse) -> Option<Self::Signature> {
        let SignResponse::Ed25519 { signature } = sig else {
            return None;
        };
        Some(Signature::from_bytes(&signature))
    }
}
