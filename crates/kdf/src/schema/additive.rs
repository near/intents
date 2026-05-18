use core::ops::Add;

use defuse_kdf_crypto::Curve;
use impl_tools::autoimpl;

use crate::Schema;

#[autoimpl(Debug, Clone where C::PublicKey: trait)]
#[derive(Copy)]
pub struct Additive<C: Curve> {
    master_pk: C::PublicKey,
}

impl<C> Additive<C>
where
    C: Curve,
{
    #[inline]
    pub const fn new(master_pk: C::PublicKey) -> Self {
        Self { master_pk }
    }

    #[inline]
    pub const fn public_key(&self) -> &C::PublicKey {
        &self.master_pk
    }
}

impl<C> Schema<C::Scalar> for Additive<C>
where
    C: CurveArithmetics,
{
    type Output = C::PublicKey;

    fn derive_path(&self, tweak: C::Scalar) -> Self::Output {
        let master_pk = C::pk2point(&self.master_pk);

        // pk' <- pk + G * tweak
        let derived_point = master_pk + C::mul_by_generator(&tweak);

        C::point2pk(derived_point)
    }
}

// TODO: docs
pub trait CurveArithmetics: Curve {
    type Scalar;
    type Point: Add<Output = Self::Point>;

    fn mul_by_generator(scalar: &Self::Scalar) -> Self::Point;
    fn pk2point(public_key: &Self::PublicKey) -> Self::Point;
    fn point2pk(point: Self::Point) -> Self::PublicKey;
}
