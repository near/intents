use core::marker::PhantomData;

use impl_tools::autoimpl;

use crate::{CurveArithmetic, Schema};

/// [`Schema`] for converting fixed byte arrays into a scalar
/// via modular reduction.
#[autoimpl(Debug, Clone, Copy, Default)]
pub struct ReduceScalar<C>(PhantomData<C>);

impl<C> ReduceScalar<C> {
    #[inline]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// A curve that supports constructing its [`Scalar`](CurveArithmetic::Scalar)
/// from a larger value `P` via modular reduction. Used by [`ReduceScalar<C>`]
/// schema.
pub trait ReducableScalarCurve<P>: CurveArithmetic {
    fn reduce(path: P) -> Self::Scalar;
}

impl<P, C> Schema<P> for ReduceScalar<C>
where
    C: ReducableScalarCurve<P>,
{
    type Output = C::Scalar;

    #[inline]
    fn derive(&self, path: P) -> Self::Output {
        <C as ReducableScalarCurve<P>>::reduce(path)
    }
}
