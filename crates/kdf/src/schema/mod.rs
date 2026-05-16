#[cfg(feature = "borsh")]
pub mod borsh;
#[cfg(feature = "digest")]
pub mod digest;
#[cfg(feature = "hex")]
pub mod hex;

use std::{rc::Rc, sync::Arc};

use impl_tools::autoimpl;

/// A generic closure that can used for [tweak](crate::DerivableCurve::Tweak)
/// derivation and its intermediary steps.
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait DerivationSchema<P> {
    /// [Derivation](DerivationSchema::derive_path) output.
    type Output;

    /// Derive output for given `path`.
    fn derive_path(&self, path: P) -> Self::Output;
}

// TODO: type params
pub trait DeriveExt {
    #[inline]
    fn map<S>(self, then: S) -> Map<Self, S>
    where
        Self: Sized,
    {
        Map(self, then)
    }

    fn map_fn<F>(self, f: F) -> Map<Self, SchemaFn<F>>
    where
        Self: Sized,
    {
        Map(self, SchemaFn::new(f))
    }

    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<S> DeriveExt for S {}

// TODO: docs, derives
#[derive(Default)]
pub struct Identity;

impl<T> DerivationSchema<T> for Identity {
    type Output = T;

    #[inline]
    fn derive_path(&self, path: T) -> T {
        path
    }
}

// TODO: docs, derives
// TODO: implement DerivableSigner, too?
#[derive(Default)]
pub struct Map<A, B>(pub(crate) A, pub(crate) B);

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
