use core::ops::Add;

use defuse_kdf_crypto::Curve;
use impl_tools::autoimpl;

use crate::Schema;

/// Additive derivation [schema](Schema).
///
/// The derivation is **non-hierarchical** (or "plain"): derived
/// keys **do not** form a tree-like structure. Instead, child keys
/// are all derived from a single root key and can be considered as
/// "peers" to each other.
#[autoimpl(Debug, Clone, Copy where C::PublicKey: trait)]
pub struct Additive<C: Curve> {
    master_pk: C::PublicKey,
}

impl<C> Additive<C>
where
    C: Curve,
{
    /// Create schema from given master public key.
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
    C: CurveArithmetic,
{
    type Output = C::PublicKey;

    fn derive_path(&self, tweak: C::Scalar) -> Self::Output {
        let master_pk = C::pk2point(&self.master_pk);

        // pk' <- pk + G * tweak
        let derived_point = master_pk + C::mul_by_generator(&tweak);

        C::point2pk(derived_point)
    }
}

/// Curve arithmetics used by [`Additive`] schema.
pub trait CurveArithmetic: Curve {
    /// Reduced scalar
    type Scalar;

    /// Point on a curve
    type Point: Add<Output = Self::Point>;

    /// Multiply given scalar by a generator (i.e. base) point of the curve.
    fn mul_by_generator(scalar: &Self::Scalar) -> Self::Point;

    /// Convert public key into a point
    fn pk2point(public_key: &Self::PublicKey) -> Self::Point;

    /// Convert a point to a public key
    fn point2pk(point: Self::Point) -> Self::PublicKey;
}
