use defuse_kdf::crypto::RecoverableCurve;
use defuse_mpc_kdf::NearMpcCurve;

use crate::contract::{Payload, PublicKey, SignResponse};

/// A [`Curve`](defuse_kdf::crypto::Curve) supported by Near MPC on-chain API.
pub trait OnChainNearMpcCurve: NearMpcCurve {
    /// Try to extract public key associated with this curve from `public_key()`
    /// method of MPC contract
    fn extract_public_key(public_key: PublicKey) -> Option<Self::PublicKey>;
    /// Try to convert message to payload for `sign()` method of MPC contract
    fn msg_to_payload(msg: &[u8]) -> Option<Payload<'_>>;
    /// Try to extract signature associated with this curve returned from
    /// `sign()` method of MPC contract
    fn extract_signature(sig: SignResponse) -> Option<Self::Signature>;
}

/// A [`RecoverableCurve`] supported by Near MPC on-chain API.
pub trait RecoverableOnChainNearMpcCurve: OnChainNearMpcCurve + RecoverableCurve {
    /// Try to extract recoverable signature associated with this curve returned
    /// from `sign()` method of MPC contract
    fn parse_recoverable_signature(
        sig: SignResponse,
    ) -> Option<(Self::Signature, Self::RecoveryId)>;
}
