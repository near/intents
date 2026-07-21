use defuse_mpc_kdf::crypto::secp256k1::{Secp256k1, Secp256k1RecoverableSignature};

use crate::{
    OnChainNearMpcCurve, RecoverableOnChainNearMpcCurve,
    contract::{Payload, PublicKey, SignResponse},
};

impl OnChainNearMpcCurve for Secp256k1 {
    #[inline]
    fn extract_public_key(public_key: PublicKey) -> Option<Self::PublicKey> {
        let PublicKey::Secp256k1(pk) = public_key else {
            return None;
        };
        pk.try_into().ok()
    }

    #[inline]
    fn msg_to_payload(prehash: &[u8]) -> Option<Payload<'_>> {
        <[u8; 32]>::try_from(prehash).map(Payload::Ecdsa).ok()
    }

    #[inline]
    fn extract_signature(sig: SignResponse) -> Option<Self::Signature> {
        Self::parse_recoverable_signature(sig).map(|s| s.0)
    }
}

impl RecoverableOnChainNearMpcCurve for Secp256k1 {
    #[inline]
    fn parse_recoverable_signature(
        sig: SignResponse,
    ) -> Option<(Self::Signature, Self::RecoveryId)> {
        let SignResponse::Secp256k1(sig) = sig else {
            return None;
        };

        let sig = Secp256k1RecoverableSignature({
            let mut buf = [0u8; 65];
            buf[..32].copy_from_slice(&sig.big_r.affine_point[1..]);
            buf[32..64].copy_from_slice(&sig.s.scalar);
            buf[64] = sig.recovery_id;
            buf
        });

        sig.try_into().ok()
    }
}
