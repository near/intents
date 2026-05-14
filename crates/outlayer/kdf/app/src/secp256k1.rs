use defuse_outlayer_kdf::crypto::secp256k1::{
    Secp256k1,
    k256::{NonZeroScalar, U256, elliptic_curve::ops::Reduce},
};

use crate::AppDerivableCurveDomain;

impl AppDerivableCurveDomain for Secp256k1 {
    const DOMAIN_SEPARATOR: &'static [u8] = b"outlayer/secp256k1/derive-tweak/v1";

    fn tweak(path: [u8; 32]) -> NonZeroScalar {
        <NonZeroScalar as Reduce<U256>>::reduce_bytes(&path.into())
    }
}
