use defuse_outlayer_kdf::crypto::ed25519::{Ed25519, Scalar};

use crate::AppDerivableCurveDomain;

impl AppDerivableCurveDomain for Ed25519 {
    const DOMAIN_SEPARATOR: &'static [u8] = b"outlayer/ed25519/derive-tweak/v1";

    fn tweak(path: [u8; 32]) -> Scalar {
        Scalar::from_bytes_mod_order(path)
    }
}
