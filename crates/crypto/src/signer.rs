use std::{
    fmt::{Debug, Display},
    sync::{Arc, OnceLock},
};

use impl_tools::autoimpl;

use crate::{AsyncRecoverableSigner, AsyncSigner, Curve, RecoverableCurve};

/// A type that knows the [public key](Curve::PublicKey) its signatures
/// verify against.
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait SignerPublicKey<C: Curve> {
    /// Public key of the signer.
    fn public_key(&self) -> C::PublicKey;

    /// Wrap `self` into [`CachePublicKey`], so that
    /// [`public_key()`](Self::public_key) is computed at most once.
    #[inline]
    fn cache_public_key(self) -> CachePublicKey<C, Self>
    where
        Self: Sized,
    {
        CachePublicKey::new(self)
    }
}

/// A signer capable of producing signatures for a specific [`Curve`].
///
/// See [`AsyncSigner`] for signers that need to await, e.g. remote ones.
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait Signer<C: Curve>: SignerPublicKey<C> {
    /// An error that can occur during [signing](Self::sign).
    type Error: Debug + Display;

    /// Sign a given message and return a signature.
    ///
    /// NOTE: implementations MAY require `msg` to be prehash (i.e. output
    /// of cryptographic hash function) of a fixed length and return
    /// an error otherwise. Check corresponding docs before using.
    fn sign(&self, msg: &[u8]) -> Result<C::Signature, Self::Error>;
}

/// A [`Signer`] that can produce [recoverable](RecoverableCurve::recover)
/// signatures.
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait RecoverableSigner<C: RecoverableCurve>: Signer<C> {
    /// Sign a given message and return a signature along with recovery id.
    ///
    /// NOTE: implementations MAY require `msg` to be prehash (i.e. output
    /// of cryptographic hash function) of a fixed length and return
    /// an error otherwise. Check corresponding docs before using.
    fn sign_recoverable(&self, msg: &[u8]) -> Result<(C::Signature, C::RecoveryId), Self::Error>;
}

/// TODO: docs
#[autoimpl(Deref using self.signer)]
#[autoimpl(Debug, Clone, PartialEq, Eq where C::PublicKey: trait, S: trait)]
pub struct CachePublicKey<C: Curve, S> {
    public_key: OnceLock<C::PublicKey>,
    signer: S,
}

impl<C: Curve, S> CachePublicKey<C, S> {
    #[inline]
    const fn new(signer: S) -> Self {
        Self {
            public_key: OnceLock::new(),
            signer,
        }
    }
}

impl<C, S> SignerPublicKey<C> for CachePublicKey<C, S>
where
    C: Curve,
    C::PublicKey: Clone,
    S: SignerPublicKey<C>,
{
    fn public_key(&self) -> C::PublicKey {
        self.public_key
            .get_or_init(|| self.signer.public_key())
            .clone()
    }
}

impl<C, S> Signer<C> for CachePublicKey<C, S>
where
    C: Curve,
    C::PublicKey: Clone + Send + Sync,
    S: Signer<C>,
{
    type Error = S::Error;

    fn sign(&self, msg: &[u8]) -> Result<C::Signature, Self::Error> {
        self.signer.sign(msg)
    }
}

impl<C, S> RecoverableSigner<C> for CachePublicKey<C, S>
where
    C: RecoverableCurve,
    C::PublicKey: Clone + Send + Sync,
    S: RecoverableSigner<C>,
{
    fn sign_recoverable(
        &self,
        msg: &[u8],
    ) -> Result<(<C>::Signature, <C as RecoverableCurve>::RecoveryId), Self::Error> {
        self.signer.sign_recoverable(msg)
    }
}

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

/// Test helpers
#[cfg(test)]
#[allow(dead_code, clippy::redundant_pub_crate)]
pub(crate) mod tests {
    use std::fmt::Debug;

    use super::*;

    pub fn test_sign_verify<C: Curve, S: Signer<C>>(signer: S, msg: impl AsRef<[u8]>) {
        let msg = msg.as_ref();
        let signature = signer.sign(msg).unwrap();
        assert!(
            C::verify(&signer.public_key(), msg, &signature),
            "signer produced invalid signature"
        );
    }

    pub fn test_sign_recover<C, S>(signer: S, msg: impl AsRef<[u8]>)
    where
        C: RecoverableCurve<PublicKey: PartialEq + Debug>,
        S: RecoverableSigner<C>,
    {
        let msg = msg.as_ref();
        let (signature, recovery_id) = signer.sign_recoverable(msg).unwrap();

        assert_eq!(
            C::recover(msg, &signature, recovery_id),
            Some(signer.public_key()),
            "can't recover signer's public key"
        );
    }
}
