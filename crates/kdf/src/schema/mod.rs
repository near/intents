mod additive;
#[cfg(feature = "borsh")]
pub mod borsh;
#[cfg(feature = "digest")]
pub mod digest;
#[cfg(feature = "hex")]
pub mod hex;
mod reduce;

pub use self::{additive::*, reduce::*};

use std::{rc::Rc, sync::Arc};

use impl_tools::autoimpl;

#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
/// A generic closure that can used for public key
/// [derivation](crate::DeriveSigner::derive_public_key) and its intermediary
/// steps.
pub trait Schema<P> {
    /// [Derivation](Schema::derive_path) output.
    type Output;

    /// Derive output for given `path`.
    fn derive_path(&self, path: P) -> Self::Output;
}

/// Helper trait with extensions for [`Schema`] and
/// [`crate::DeriveSigner`]
pub trait DeriveExt {
    /// Derive with given [schema](Schema).
    ///
    /// ```rust
    /// use defuse_kdf::{DeriveExt, Schema, SchemaFn};
    ///
    /// let schema_a = SchemaFn::new(|v| v + 1);
    /// let schema_b = SchemaFn::new(|v| v * 2);
    ///
    /// let schema_ab = schema_a.derive(schema_b);
    ///
    /// assert_eq!(schema_ab.derive_path(3), 7);
    /// ```
    #[inline]
    fn derive<D>(self, with: D) -> Derive<Self, D>
    where
        Self: Sized,
    {
        Derive(self, with)
    }

    /// Creates "by reference" adaptor.
    #[inline]
    fn by_ref(&self) -> &Self {
        self
    }
}
impl<S> DeriveExt for S {}

/// No-op identity adator for [`Schema`].
///
/// ```rust
/// use defuse_kdf::{Schema, Identity};
///
/// assert_eq!(Identity.derive_path(42), 42);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Identity;

impl<T> Schema<T> for Identity {
    type Output = T;

    #[inline]
    fn derive_path(&self, path: T) -> T {
        path
    }
}

/// Derive adaptor for [`Schema`] and [`crate::DeriveSigner`].
/// See [`.derive()`](DeriveExt::derive).
#[derive(Debug, Clone, Copy, Default)]
pub struct Derive<S, D>(pub(crate) S, pub(crate) D);

impl<P, S, D> Schema<P> for Derive<S, D>
where
    D: Schema<P>,
    S: Schema<D::Output>,
{
    type Output = S::Output;

    #[inline]
    fn derive_path(&self, path: P) -> Self::Output {
        self.0.derive_path(self.1.derive_path(path))
    }
}

/// Adaptor for creating [`Schema`] from a closure.
///
/// ```rust
/// use defuse_kdf::{Schema, SchemaFn};
///
/// let schema = SchemaFn::new(|v| v + 2);
///
/// assert_eq!(schema.derive_path(3), 5);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SchemaFn<F>(F);

impl<F> SchemaFn<F> {
    #[inline]
    pub const fn new(f: F) -> Self {
        Self(f)
    }
}

impl<P, F, O> Schema<P> for SchemaFn<F>
where
    F: Fn(P) -> O,
{
    type Output = O;

    #[inline]
    fn derive_path(&self, path: P) -> Self::Output {
        (self.0)(path)
    }
}

/// [`Schema`] adaptor that always uses the same path by cloning it.
///
/// ```rust
/// use defuse_kdf::{Path, Schema};
///  
/// let schema = Path::new("abc");
///
/// assert_eq!(schema.derive_path(()), "abc");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Path<P>(P);

impl<P> Path<P> {
    #[inline]
    pub const fn new(path: P) -> Self {
        Self(path)
    }

    #[inline]
    pub fn into_inner(self) -> P {
        self.0
    }
}

impl<P> Schema<()> for Path<P>
where
    P: Clone,
{
    type Output = P;

    fn derive_path(&self, _path: ()) -> Self::Output {
        self.0.clone()
    }
}

impl<P> AsRef<P> for Path<P> {
    fn as_ref(&self) -> &P {
        &self.0
    }
}

pub type BoxSchema<'a, P, O> = Box<dyn Schema<P, Output = O> + 'a>;
