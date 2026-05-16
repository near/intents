#[cfg(feature = "borsh")]
pub mod borsh;
#[cfg(feature = "digest")]
pub mod digest;
#[cfg(feature = "hex")]
pub mod hex;

use std::{borrow::Cow, rc::Rc, sync::Arc};

use impl_tools::autoimpl;

#[autoimpl(for<T: trait + ?Sized + ToOwned> Cow<'_, T>)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait DerivationSchema<P> {
    type Output;

    fn derive_path(&self, path: P) -> Self::Output;
}

// TODO: type params
pub trait DerivationSchemaExt<P>: DerivationSchema<P> {
    #[inline]
    fn map<S>(self, then: S) -> Map<Self, S>
    where
        Self: Sized,
    {
        Map(self, then)
    }
}

impl<S, P> DerivationSchemaExt<P> for S where S: DerivationSchema<P> {}

#[derive(Default)]
pub struct Identity;

impl<T> DerivationSchema<T> for Identity {
    type Output = T;

    #[inline]
    fn derive_path(&self, path: T) -> T {
        path
    }
}

// TODO: implement DerivableSigner, too?
#[derive(Default)]
pub struct Map<A, B>(A, B);

impl<A, B> Map<A, B> {
    pub const fn new(first: A, second: B) -> Self {
        Self(first, second)
    }
}

impl<P, A, B> DerivationSchema<P> for Map<A, B>
where
    A: DerivationSchema<P>,
    B: DerivationSchema<A::Output>,
{
    type Output = B::Output;

    fn derive_path(&self, path: P) -> Self::Output {
        self.1.derive_path(self.0.derive_path(path))
    }
}

pub struct SchemaFn<F>(F);

impl<F> SchemaFn<F> {
    #[inline]
    pub const fn new(f: F) -> Self {
        Self(f)
    }
}

impl<P, F, O> DerivationSchema<P> for SchemaFn<F>
where
    F: Fn(P) -> O,
{
    type Output = O;

    fn derive_path(&self, path: P) -> Self::Output {
        (self.0)(path)
    }
}
