use defuse_kdf_crypto::Secp256k1;
use k256::{
    NonZeroScalar, ProjectivePoint, U256,
    ecdsa::VerifyingKey,
    elliptic_curve::{
        bigint::U512,
        ops::{MulByGenerator, Reduce},
    },
};

use crate::Schema;

use super::{CurveArithmetics, ReduceScalar};

impl CurveArithmetics for Secp256k1 {
    type Scalar = NonZeroScalar;

    type Point = ProjectivePoint;

    fn mul_by_generator(scalar: &Self::Scalar) -> Self::Point {
        ProjectivePoint::mul_by_generator(scalar)
    }

    fn pk2point(public_key: &Self::PublicKey) -> Self::Point {
        public_key.as_affine().into()
    }

    fn point2pk(point: Self::Point) -> Self::PublicKey {
        VerifyingKey::from_affine(point.to_affine())
            .expect("derived public key is the point at infinity")
    }
}

impl Schema<[u8; 32]> for ReduceScalar<Secp256k1> {
    type Output = NonZeroScalar;

    #[inline]
    fn derive_path(&self, path: [u8; 32]) -> Self::Output {
        Reduce::<U256>::reduce_bytes(&path.into())
    }
}

impl Schema<[u8; 64]> for ReduceScalar<Secp256k1> {
    type Output = NonZeroScalar;

    #[inline]
    fn derive_path(&self, path: [u8; 64]) -> Self::Output {
        Reduce::<U512>::reduce_bytes(&path.into())
    }
}
