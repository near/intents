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

/// A generic closure that can used for public key
/// [derivation](crate::DeriveSigner::derive_public_key) and its intermediary
/// steps.
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Rc<T>, Arc<T>)]
pub trait Schema<P> {
    /// [Derivation](Schema::derive) output.
    type Output;

    /// Derive output for given `path`.
    fn derive(&self, path: P) -> Self::Output;

    /// Derive with given [schema](Schema).
    ///
    /// ```rust
    /// use defuse_kdf::{Schema, SchemaFn};
    ///
    /// let schema_a = SchemaFn::new(|v| v + 1);
    /// let schema_b = SchemaFn::new(|v| v * 2);
    ///
    /// let schema_ab = schema_a.derive_with(schema_b);
    ///
    /// assert_eq!(schema_ab.derive_path(3), 7);
    /// ```
    #[inline]
    fn derive_with<D>(self, with: D) -> Derive<Self, D>
    where
        Self: Sized,
    {
        Derive(self, with)
    }
}

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
    fn derive(&self, path: T) -> T {
        path
    }
}

/// Derive adaptor for [`Schema`] and [`crate::DeriveSigner`].
/// See [`.derive()`](Derive::derive).
#[derive(Debug, Clone, Copy, Default)]
pub struct Derive<S, D>(pub(crate) S, pub(crate) D);

impl<P, S, D> Schema<P> for Derive<S, D>
where
    D: Schema<P>,
    S: Schema<D::Output>,
{
    type Output = S::Output;

    #[inline]
    fn derive(&self, path: P) -> Self::Output {
        self.0.derive(self.1.derive(path))
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
    fn derive(&self, path: P) -> Self::Output {
        (self.0)(path)
    }
}

/// [`Schema`] adaptor that always uses the same path by cloning it.
///
/// ```rust
/// use defuse_kdf::{Value, Schema};
///  
/// let schema = Value::new("abc");
///
/// assert_eq!(schema.derive_path(()), "abc");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Value<P>(P);

impl<P> Value<P> {
    #[inline]
    pub const fn new(path: P) -> Self {
        Self(path)
    }

    #[inline]
    pub fn into_inner(self) -> P {
        self.0
    }
}

impl<P> Schema<()> for Value<P>
where
    P: Clone,
{
    type Output = P;

    fn derive(&self, _path: ()) -> Self::Output {
        self.0.clone()
    }
}

impl<P> AsRef<P> for Value<P> {
    fn as_ref(&self) -> &P {
        &self.0
    }
}

pub type BoxSchema<'a, P, O> = Box<dyn Schema<P, Output = O> + 'a>;
