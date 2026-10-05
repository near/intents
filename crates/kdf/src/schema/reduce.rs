use core::marker::PhantomData;

use impl_tools::autoimpl;

use crate::{CurveArithmetic, Schema};

/// [`Schema`](crate::Schema) for converting fixed byte arrays into a scalar
/// via modular reduction.
#[autoimpl(Debug, Clone, Copy, Default)]
pub struct ReduceScalar<C>(PhantomData<C>);

impl<C> ReduceScalar<C> {
    #[inline]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// A [`Scalar`](CurveArithmetic::Scalar) that can constructed from a larger
/// value `P` via modular reduction
pub trait ReducableScalar<P>: Sized {
    fn reduce(path: P) -> Self;
}

impl<P, C> Schema<P> for ReduceScalar<C>
where
    C: CurveArithmetic<Scalar: ReducableScalar<P>>,
{
    type Output = C::Scalar;

    #[inline]
    fn derive(&self, path: P) -> Self::Output {
        <C::Scalar as ReducableScalar<P>>::reduce(path)
    }
}
