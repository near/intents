use curve25519_dalek::{EdwardsPoint, Scalar};
use defuse_kdf_crypto::Ed25519;

use crate::Schema;

use super::{CurveArithmetics, ReduceScalar};

impl CurveArithmetics for Ed25519 {
    type Scalar = Scalar;

    type Point = EdwardsPoint;

    fn mul_by_generator(scalar: &Self::Scalar) -> Self::Point {
        EdwardsPoint::mul_base(&scalar)
    }

    fn pk2point(public_key: &Self::PublicKey) -> Self::Point {
        public_key.to_edwards()
    }

    fn point2pk(point: Self::Point) -> Self::PublicKey {
        point.into()
    }
}

impl Schema<[u8; 32]> for ReduceScalar<Ed25519> {
    type Output = Scalar;

    #[inline]
    fn derive_path(&self, path: [u8; 32]) -> Self::Output {
        Scalar::from_bytes_mod_order(path)
    }
}

impl Schema<[u8; 64]> for ReduceScalar<Ed25519> {
    type Output = Scalar;

    #[inline]
    fn derive_path(&self, path: [u8; 64]) -> Self::Output {
        Scalar::from_bytes_mod_order_wide(&path)
    }
}
