use defuse_crypto::{Curve, RecoverableCurve};

use crate::{
    DeriveSigner, RecoverableDeriveSigner,
    schema::{Schema, Value},
};

/// A helper extension trait for [`Schema`] and [`DeriveSigner`].
pub trait DeriveExt {
    /// Derive with given inner sub-[schema](Schema).
    ///
    /// ```rust
    /// use defuse_kdf::{DeriveExt, Schema, SchemaFn};
    ///
    /// let schema_a = SchemaFn::new(|v| v + 1);
    /// let schema_b = SchemaFn::new(|v| v * 2);
    ///
    /// let schema_ab = schema_a.derive_with(schema_b);
    ///
    /// assert_eq!(schema_ab.derive(3), 7);
    /// ```
    #[inline]
    fn derive_with<D>(self, inner: D) -> Derive<Self, D>
    where
        Self: Sized,
    {
        Derive::new(self, inner)
    }

    /// Derive a signer with a given value for [current](Self::schema) schema,
    /// so that returned signer implements [`Signer`] and doesn't take any
    /// derivation path.
    ///
    // TODO: docs
    #[inline]
    fn derive_value<P>(self, value: P) -> Derive<Self, Value<P>>
    where
        Self: Sized,
    {
        self.derive_with(Value::new(value))
    }

    /// Helper method to [derive](Schema::derive) public key for given
    /// `path` via [`.schema()`](DeriveSigner::Schema)
    // TODO: doc
    #[inline]
    fn derive_public_key<C: Curve, P>(&self, path: P) -> C::PublicKey
    where
        Self: DeriveSigner<C, P>,
    {
        self.schema().derive(path)
    }

    // TODO: doc
    #[inline]
    fn derive_sign<C: Curve, P>(
        &self,
        path: P,
        msg: impl AsRef<[u8]> + Send,
    ) -> impl Future<Output = Result<C::Signature, Self::Error>> + Send
    where
        Self: DeriveSigner<C, P>,
        P: Send,
    {
        async move { DeriveSigner::<C, P>::derive_sign(self, path, msg.as_ref()).await }
    }

    // TODO: doc

    #[inline]
    fn derive_sign_recoverable<C: RecoverableCurve, P>(
        &self,
        path: P,
        msg: impl AsRef<[u8]> + Send,
    ) -> impl Future<Output = Result<(C::Signature, C::RecoveryId), Self::Error>> + Send
    where
        Self: RecoverableDeriveSigner<C, P>,
        P: Send,
    {
        async move {
            RecoverableDeriveSigner::<C, P>::derive_sign_recoverable(self, path, msg.as_ref()).await
        }
    }
}
impl<T> DeriveExt for T {}

/// Derive adaptor for [`Schema`] and [`DeriveSigner`].
///
/// See [`.derive_with()`](DeriveExt::derive_with).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Derive<S, D> {
    pub(crate) outer: S,
    pub(crate) inner: D,
}

impl<S, D> Derive<S, D> {
    #[must_use]
    #[inline]
    pub const fn new(outer: S, inner: D) -> Self {
        Self { outer, inner }
    }
}
