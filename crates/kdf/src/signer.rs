use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use defuse_crypto::{Curve, RecoverableCurve, RecoverableSigner, Signer, SignerPublicKey};
use impl_tools::autoimpl;

use crate::{Derive, DeriveExt, Schema, Value};

/// Public key derivation [schema](Schema) of a signer that **internally**
/// derives its signing keys.
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait DeriveSignerSchema<C: Curve, P> {
    /// [`Schema`] for public key derivation.
    /// See [`.schema()`](Self::schema) for details.
    type Schema<'a>: Schema<P, Output = C::PublicKey>
    where
        Self: 'a;

    /// Construct [schema](Schema) for public key derivation.
    ///
    /// For _non-hardened_ (i.e. "public") derivation, the returned schema
    /// shouldn't contain any secret information, so that derivation can be
    /// performed by clients fully offline, without any interactions with the
    /// signer, since all parameters are public.
    ///
    /// For _hardened_ derivation, this would typically reference `self`, since
    /// public keys can be only derived by knowing a master signing key.
    ///
    /// See [`.derive_public_key()`](Self::derive_public_key) shorthand.
    fn schema(&self) -> Self::Schema<'_>;

    /// Helper method to [derive](Schema::derive) public key for given
    /// `path` via [`.schema()`](Self::schema)
    #[inline]
    fn derive_public_key(&self, path: P) -> C::PublicKey {
        self.schema().derive(path)
    }

    /// Derive a signer with a given value for [current](Self::schema) schema,
    /// so that returned signer implements [`Signer`] and doesn't take any
    /// derivation path.
    #[inline]
    fn derive(self, value: P) -> Derive<Self, Value<P>>
    where
        Self: Sized,
    {
        self.derive_with(Value::new(value))
    }
}

/// A signer that can sign messages by **internally** deriving signing keys
/// according to its public key derivation
/// [schema](DeriveSignerSchema::schema).
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait DeriveSigner<C: Curve, P>: DeriveSignerSchema<C, P> {
    type Error: Debug + Display;

    /// Sign given message with a secret key **internally** derived
    /// for given `path` according to
    /// [`schema`](DeriveSignerSchema::schema).
    ///
    /// **NOTE**:
    /// * Implementations MAY require `msg` to be prehash (i.e. output
    ///   of cryptographic hash function) of a fixed length and fail
    ///   otherwise. Check corresponding docs before using.
    /// * The returned signatures MIGHT be non-deterministic, i.e.
    ///   implementations MAY return different signatures for the same
    ///   `path` and `msg`.
    fn derive_sign(&self, path: P, msg: &[u8]) -> Result<C::Signature, Self::Error>;
}

/// A [signer](DeriveSigner) that can recoverably sign messages by
/// **internally** deriving signing keys.
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait RecoverableDeriveSigner<C: RecoverableCurve, P>: DeriveSigner<C, P> {
    /// Recoverably [sign](DeriveSigner::derive_sign) given message with a
    /// secret key **internally** derived for given `path` according to
    /// [`schema`](DeriveSignerSchema::schema) and return
    /// [signature](Curve::Signature) along with
    /// [recovery id](RecoverableCurve::RecoveryId).
    ///
    /// **NOTE**:
    /// * Implementations MAY require `msg` to be prehash (i.e. output
    ///   of cryptographic hash function) of a fixed length and fail
    ///   otherwise. Check corresponding docs before using.
    /// * The returned signatures MIGHT be non-deterministic, i.e.
    ///   implementations MAY return different signatures for the same
    ///   `path` and `msg`.
    fn derive_sign_recoverable(
        &self,
        path: P,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error>;
}

impl<C, P, S, D> DeriveSignerSchema<C, P> for Derive<S, D>
where
    C: Curve,
    S: DeriveSignerSchema<C, D::Output>,
    D: Schema<P>,
{
    type Schema<'a>
        = Derive<S::Schema<'a>, &'a D>
    where
        Self: 'a;

    #[inline]
    fn schema(&self) -> Self::Schema<'_> {
        self.outer.schema().derive_with(&self.inner)
    }
}

impl<C, P, S, D> DeriveSigner<C, P> for Derive<S, D>
where
    C: Curve,
    S: DeriveSigner<C, D::Output>,
    D: Schema<P>,
{
    type Error = S::Error;

    fn derive_sign(&self, path: P, msg: &[u8]) -> Result<C::Signature, Self::Error> {
        self.outer.derive_sign(self.inner.derive(path), msg)
    }
}

impl<C, P, S, D> RecoverableDeriveSigner<C, P> for Derive<S, D>
where
    C: RecoverableCurve,
    S: RecoverableDeriveSigner<C, D::Output>,
    D: Schema<P>,
{
    fn derive_sign_recoverable(
        &self,
        path: P,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error> {
        self.outer
            .derive_sign_recoverable(self.inner.derive(path), msg)
    }
}

impl<C, S, D> SignerPublicKey<C> for Derive<S, D>
where
    C: Curve,
    S: DeriveSignerSchema<C, D::Output>,
    D: Schema<()>,
{
    #[inline]
    fn public_key(&self) -> <C as Curve>::PublicKey {
        self.derive_public_key(())
    }
}

impl<C, S, D> Signer<C> for Derive<S, D>
where
    C: Curve,
    S: DeriveSigner<C, D::Output>,
    D: Schema<()>,
{
    type Error = S::Error;

    fn sign(&self, msg: &[u8]) -> Result<C::Signature, Self::Error> {
        DeriveSigner::<C, _>::derive_sign(self, (), msg)
    }
}

impl<C, S, D> RecoverableSigner<C> for Derive<S, D>
where
    C: RecoverableCurve,
    S: RecoverableDeriveSigner<C, D::Output>,
    D: Schema<()>,
{
    fn sign_recoverable(&self, msg: &[u8]) -> Result<(C::Signature, C::RecoveryId), Self::Error> {
        RecoverableDeriveSigner::<C, _>::derive_sign_recoverable(self, (), msg)
    }
}

#[cfg(any(test, feature = "testing"))]
pub fn assert_signer_roundtrip<C, S, P>(
    signer: &S,
    path: P,
    msg: &[u8],
) -> (C::PublicKey, C::Signature)
where
    C: Curve,
    S: DeriveSigner<C, P>,
    P: Clone,
{
    let derived_pk = signer.derive_public_key(path.clone());
    let signature = signer.derive_sign(path, msg).expect("failed to sign");

    assert!(C::verify(&derived_pk, msg, &signature), "invalid signature");

    (derived_pk, signature)
}
