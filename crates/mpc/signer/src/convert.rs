use defuse_kdf::crypto::RecoverableCurve;
use defuse_mpc_kdf::NearMpcCurve;

use crate::contract::{Payload, PublicKey, SignResponse};

// TODO: docs
pub trait OnChainNearMpcCurve: NearMpcCurve {
    fn parse_public_key(public_key: PublicKey) -> Option<Self::PublicKey>;
    fn to_payload(msg: &[u8]) -> Option<Payload<'_>>;
    fn parse_signature(sig: SignResponse) -> Option<Self::Signature>;
}

pub trait RecoverableOnChainNearMpcCurve: OnChainNearMpcCurve + RecoverableCurve {
    fn parse_recoverable_signature(
        sig: SignResponse,
    ) -> Option<(Self::Signature, Self::RecoveryId)>;
}
