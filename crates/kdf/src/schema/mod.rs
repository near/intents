#[cfg(feature = "borsh")]
pub mod borsh;
#[cfg(feature = "digest")]
pub mod digest;
#[cfg(feature = "hex")]
pub mod hex;

use std::{rc::Rc, sync::Arc};

use impl_tools::autoimpl;

#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
/// A generic closure that can used for [tweak](crate::DerivableCurve::Tweak)
/// derivation and its intermediary steps.
pub trait DerivationSchema<P> {
    /// [Derivation](DerivationSchema::derive_path) output.
    type Output;

    /// Derive output for given `path`.
    fn derive_path(&self, path: P) -> Self::Output;
}

/// Helper trait with extensions for [`DerivationSchema`] and
/// [`crate::DeriveSigner`]
pub trait DeriveExt {
    #[inline]
    fn map<S>(self, then: S) -> Map<Self, S>
    where
        Self: Sized,
    {
        Map(self, then)
    }

    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<S> DeriveExt for S {}

#[derive(Default, Clone, Copy)]
/// No-op identity adator for [`DerivationSchema`].
///
/// ```rust
/// use defuse_kdf::{DerivationSchema, Identity};
///
/// assert_eq!(Identity.derive_path(42), 42);
/// ```
pub struct Identity;

impl<T> DerivationSchema<T> for Identity {
    type Output = T;

    #[inline]
    fn derive_path(&self, path: T) -> T {
        path
    }
}

#[derive(Default, Clone, Copy)]
/// Mapping adaptor for [`DerivationSchema`] and [`crate::DeriveSigner`].
///
/// ```rust
/// use defuse_kdf::{DeriveExt, DerivationSchema, SchemaFn};
///
/// let schema_a = SchemaFn::new(|v| v * 2);
/// let schema_b = SchemaFn::new(|v| v + 1);
///
/// let schema_ab = schema_a.map(schema_b);
///
/// assert_eq!(schema_ab.derive_path(3), 7);
/// ```
pub struct Map<A, B>(pub(crate) A, pub(crate) B);

impl<P, A, B> DerivationSchema<P> for Map<A, B>
where
    A: DerivationSchema<P>,
    B: DerivationSchema<A::Output>,
{
    type Output = B::Output;

    #[inline]
    fn derive_path(&self, path: P) -> Self::Output {
        self.1.derive_path(self.0.derive_path(path))
    }
}

/// Adaptor for creating [`DerivationSchema`] from a closure.
///
/// ```rust
/// use defuse_kdf::{DerivationSchema, SchemaFn};
///
/// let schema = SchemaFn::new(|v| v + 2);
///
/// assert_eq!(schema.derive_path(3), 5);
/// ```
#[derive(Clone, Copy)]
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

    #[inline]
    fn derive_path(&self, path: P) -> Self::Output {
        (self.0)(path)
    }
}
