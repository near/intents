use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use defuse_crypto::{
    AsAsync, AsyncRecoverableSigner, AsyncSigner, Curve, RecoverableCurve, SignerPublicKey,
};
use impl_tools::autoimpl;

use crate::{Derive, DeriveSigner, DeriveSignerSchema, RecoverableDeriveSigner, Schema};

/// An asynchronous [`DeriveSigner`].
#[trait_variant::make(Send)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait AsyncDeriveSigner<C: Curve, P>: DeriveSignerSchema<C, P> + Sync {
    type Error: Debug + Display;

    /// Asynchronous [`DeriveSigner::derive_sign`].
    async fn derive_sign_async(&self, path: P, msg: &[u8]) -> Result<C::Signature, Self::Error>
    where
        P: Send;
}

/// An asynchronous [`RecoverableDeriveSigner`].
#[trait_variant::make(Send)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait AsyncRecoverableDeriveSigner<C: RecoverableCurve, P>: AsyncDeriveSigner<C, P> {
    /// Asynchronous [`RecoverableDeriveSigner::derive_sign_recoverable`].
    async fn derive_sign_recoverable_async(
        &self,
        path: P,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error>
    where
        P: Send;
}

impl<S, C, P> DeriveSignerSchema<C, P> for AsAsync<S>
where
    C: Curve,
    S: DeriveSignerSchema<C, P>,
{
    type Schema<'a>
        = S::Schema<'a>
    where
        Self: 'a;

    #[inline]
    fn schema(&self) -> Self::Schema<'_> {
        self.0.schema()
    }
}

impl<S, C, P> AsyncDeriveSigner<C, P> for AsAsync<S>
where
    S: DeriveSigner<C, P> + Send + Sync,
    C: Curve,
    P: Send,
{
    type Error = S::Error;

    async fn derive_sign_async(&self, path: P, msg: &[u8]) -> Result<C::Signature, Self::Error> {
        self.0.derive_sign(path, msg)
    }
}

impl<S, C, P> AsyncRecoverableDeriveSigner<C, P> for AsAsync<S>
where
    S: RecoverableDeriveSigner<C, P> + Send + Sync,
    C: RecoverableCurve,
    P: Send,
{
    async fn derive_sign_recoverable_async(
        &self,
        path: P,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error> {
        self.0.derive_sign_recoverable(path, msg)
    }
}

impl<C, P, S, D> AsyncDeriveSigner<C, P> for Derive<S, D>
where
    C: Curve,
    S: AsyncDeriveSigner<C, D::Output>,
    D: Schema<P, Output: Send> + Send + Sync,
{
    type Error = S::Error;

    async fn derive_sign_async(&self, path: P, msg: &[u8]) -> Result<C::Signature, Self::Error> {
        self.outer
            .derive_sign_async(self.inner.derive(path), msg)
            .await
    }
}

impl<C, P, S, D> AsyncRecoverableDeriveSigner<C, P> for Derive<S, D>
where
    C: RecoverableCurve,
    S: AsyncRecoverableDeriveSigner<C, D::Output>,
    D: Schema<P, Output: Send + Sync> + Send + Sync,
{
    async fn derive_sign_recoverable_async(
        &self,
        path: P,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error> {
        self.outer
            .derive_sign_recoverable_async(self.inner.derive(path), msg)
            .await
    }
}

impl<C, S, D> AsyncSigner<C> for Derive<S, D>
where
    C: Curve,
    S: AsyncDeriveSigner<C, D::Output>,
    D: Schema<(), Output: Send> + Send + Sync,
    Self: SignerPublicKey<C>,
{
    type Error = <S as AsyncDeriveSigner<C, D::Output>>::Error;

    #[inline]
    async fn sign_async(&self, msg: &[u8]) -> Result<C::Signature, Self::Error> {
        AsyncDeriveSigner::<C, _>::derive_sign_async(self, (), msg).await
    }
}

impl<C, S, D> AsyncRecoverableSigner<C> for Derive<S, D>
where
    C: RecoverableCurve,
    S: AsyncRecoverableDeriveSigner<C, D::Output>,
    D: Schema<(), Output: Send + Sync> + Send + Sync,
    Self: SignerPublicKey<C>,
{
    #[inline]
    async fn sign_recoverable_async(
        &self,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error> {
        AsyncRecoverableDeriveSigner::<C, _>::derive_sign_recoverable_async(self, (), msg).await
    }
}
