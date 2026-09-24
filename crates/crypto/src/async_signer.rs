use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use impl_tools::autoimpl;

use crate::{CachePublicKey, Curve, RecoverableCurve, RecoverableSigner, Signer, SignerPublicKey};

/// An asynchronous counterpart of [`Signer`], for signers that need to await
/// in order to sign, e.g. ones backed by a remote MPC network.
///
/// Use [`AsAsync`] to adapt an existing synchronous [`Signer`] to this trait.
#[trait_variant::make(Send)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait AsyncSigner<C: Curve>: SignerPublicKey<C> + Sync {
    /// An error that can occur during [signing](Self::sign).
    type Error: Debug + Display;

    /// Asynchronous [`Signer::sign`].
    async fn sign_async(&self, msg: &[u8]) -> Result<C::Signature, Self::Error>;
}

/// An [`AsyncSigner`] that can produce
/// [recoverable](RecoverableCurve::recover) signatures.
#[trait_variant::make(Send)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait AsyncRecoverableSigner<C: RecoverableCurve>: AsyncSigner<C> {
    /// Asynchronous [`RecoverableSigner::sign_recoverable`].
    async fn sign_recoverable_async(
        &self,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error>;
}

/// Adapts a synchronous [`Signer`] to [`AsyncSigner`], by resolving
/// immediately.
#[autoimpl(Deref using self.0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::From)]
pub struct AsAsync<S>(pub S);

impl<C, S> SignerPublicKey<C> for AsAsync<S>
where
    C: Curve,
    S: SignerPublicKey<C>,
{
    #[inline]
    fn public_key(&self) -> C::PublicKey {
        self.0.public_key()
    }
}

impl<C, S> AsyncSigner<C> for AsAsync<S>
where
    C: Curve,
    S: Signer<C> + Send + Sync,
{
    type Error = S::Error;

    #[inline]
    async fn sign_async(&self, msg: &[u8]) -> Result<C::Signature, Self::Error> {
        self.0.sign(msg)
    }
}

impl<C, S> AsyncRecoverableSigner<C> for AsAsync<S>
where
    C: RecoverableCurve,
    S: RecoverableSigner<C> + Send + Sync,
{
    #[inline]
    async fn sign_recoverable_async(
        &self,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error> {
        self.0.sign_recoverable(msg)
    }
}

/// Wrap `self` into [`AsAsync`]
pub trait IntoAsync: Sized {
    #[inline]
    #[must_use]
    fn into_async(self) -> AsAsync<Self> {
        AsAsync(self)
    }
}

impl<T> IntoAsync for T {}

impl<C, S> AsyncSigner<C> for CachePublicKey<C, S>
where
    C: Curve,
    C::PublicKey: Clone + Send + Sync,
    S: AsyncSigner<C>,
{
    type Error = S::Error;

    #[inline]
    async fn sign_async(&self, msg: &[u8]) -> Result<C::Signature, Self::Error> {
        self.signer.sign_async(msg).await
    }
}

impl<C, S> AsyncRecoverableSigner<C> for CachePublicKey<C, S>
where
    C: RecoverableCurve,
    C::PublicKey: Clone + Send + Sync,
    S: AsyncRecoverableSigner<C>,
{
    #[inline]
    async fn sign_recoverable_async(
        &self,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error> {
        self.signer.sign_recoverable_async(msg).await
    }
}
