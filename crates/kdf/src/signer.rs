use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use defuse_crypto::{Curve, RecoverableCurve, RecoverableSigner, Signer};
use impl_tools::autoimpl;

use crate::{Derive, DeriveExt, Schema};

/// A signer that can sign messages by **internally** deriving signing keys
/// according to its public key derivation [schema](DeriveSigner::schema).
#[trait_variant::make(Send)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait DeriveSigner<C: Curve, P>: Sync {
    type Error: Debug + Display;

    /// [`Schema`] for public key derivation.
    /// See [`.schema()`](DeriveSigner::schema) for details.
    type Schema<'a>: Schema<P, Output = C::PublicKey>
    where
        Self: 'a;

    /// Construct [schema](Schema) for public key derivation.
    ///
    /// For _non-hardened_ (i.e. "public") derivaton, the returned schema
    /// shoudn't contain any secret information, so that derivation can be
    /// performed by clients fully offline, without any interactions with the
    /// signer, since all parameters are public.
    ///
    /// For _hardened_ derivation, this would typically reference `self`, since
    /// public keys can be only derived by knowing a master signing key.
    ///
    /// See [`.derive_public_key()`](DeriveSigner::derive_public_key)
    /// shorthand.
    fn schema(&self) -> Self::Schema<'_>;

    /// Sign given message with a secret key **internally** derived
    /// for given `path` according to [`schema`](DeriveSigner::schema).
    ///
    /// **NOTE**:
    /// * Implementations MAY require `msg` to be prehash (i.e. output
    ///   of cryptographic hash function) of a fixed length and fail
    ///   otherwise. Check corresponding docs before using.
    /// * The returned signatures MIGHT be non-deterministic, i.e.
    ///   implementations MAY return different signatures for the same
    ///   `path` and `msg`.
    async fn derive_sign(&self, path: P, msg: &[u8]) -> Result<C::Signature, Self::Error>
    where
        P: Send;

    /// Helper method to [derive](Schema::derive_path) public key for given
    /// `path` via [`.schema()`](DeriveSigner::Schema)
    #[inline]
    fn derive_public_key(&self, path: P) -> C::PublicKey {
        self.schema().derive_path(path)
    }
}

/// A [signer](DeriveSigner) that can recoverably sign messages by
/// **internally** deriving signing keys.
#[trait_variant::make(Send)]
pub trait RecoverableDeriveSigner<C: RecoverableCurve, P>: DeriveSigner<C, P> {
    /// Recoveryably [sign](DeriveSigner::derive_sign) given message with a
    /// secret key **internally** derived for given `path` according to
    /// [`schema`](DeriveSigner::schema) and return [signature](Curve::Signature)
    /// along with [recovery id](RecoverableCurve::RecoveryId).
    ///
    /// **NOTE**:
    /// * Implementations MAY require `msg` to be prehash (i.e. output
    ///   of cryptographic hash function) of a fixed length and fail
    ///   otherwise. Check corresponding docs before using.
    /// * The returned signatures MIGHT be non-deterministic, i.e.
    ///   implementations MAY return different signatures for the same
    ///   `path` and `msg`.
    async fn derive_sign_recoverable(
        &self,
        path: P,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error>
    where
        P: Send;
}

impl<C, P, S, D> DeriveSigner<C, P> for Derive<S, D>
where
    C: Curve,
    S: DeriveSigner<C, D::Output>,
    D: Schema<P, Output: Send> + Send + Sync,
{
    type Error = S::Error;

    type Schema<'a>
        = Derive<S::Schema<'a>, &'a D>
    where
        Self: 'a;

    #[inline]
    fn schema(&self) -> Self::Schema<'_> {
        self.0.schema().derive(&self.1)
    }

    async fn derive_sign(&self, path: P, msg: &[u8]) -> Result<C::Signature, Self::Error>
    where
        P: Send,
    {
        self.0.derive_sign(self.1.derive_path(path), msg).await
    }
}

impl<C, P, S, D> RecoverableDeriveSigner<C, P> for Derive<S, D>
where
    C: RecoverableCurve,
    S: RecoverableDeriveSigner<C, D::Output>,
    D: Schema<P, Output: Send> + Send + Sync,
{
    async fn derive_sign_recoverable(
        &self,
        path: P,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error>
    where
        P: Send,
    {
        self.0
            .derive_sign_recoverable(self.1.derive_path(path), msg)
            .await
    }
}

impl<C, S, D> Signer<C> for Derive<S, D>
where
    C: Curve,
    S: DeriveSigner<C, D::Output>,
    D: Schema<(), Output: Send> + Send + Sync,
{
    type Error = S::Error;

    #[inline]
    fn public_key(&self) -> <C as Curve>::PublicKey {
        self.derive_public_key(())
    }

    async fn sign(&self, msg: &[u8]) -> Result<C::Signature, Self::Error> {
        DeriveSigner::<C, _>::derive_sign(self, (), msg).await
    }
}

impl<C, S, D> RecoverableSigner<C> for Derive<S, D>
where
    C: RecoverableCurve,
    S: RecoverableDeriveSigner<C, D::Output>,
    D: Schema<(), Output: Send> + Send + Sync,
{
    async fn sign_recoverable(
        &self,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error> {
        RecoverableDeriveSigner::<C, _>::derive_sign_recoverable(self, (), msg).await
    }
}

#[cfg(any(test, feature = "testing"))]
pub async fn assert_signer_roundtrip<C, S, P>(
    signer: &S,
    path: P,
    msg: &[u8],
) -> (C::PublicKey, C::Signature)
where
    C: Curve,
    S: DeriveSigner<C, P>,
    P: Clone + Send,
{
    let derived_pk = signer.derive_public_key(path.clone());
    let signature = signer.derive_sign(path, msg).await.expect("failed to sign");

    assert!(C::verify(&derived_pk, msg, &signature), "invalid signature");

    (derived_pk, signature)
}
