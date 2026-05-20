use core::marker::PhantomData;

use impl_tools::autoimpl;

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
