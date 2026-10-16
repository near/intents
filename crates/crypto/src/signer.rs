use std::{
    fmt::{Debug, Display},
    sync::{Arc, OnceLock},
};

use impl_tools::autoimpl;

use crate::{Curve, RecoverableCurve};

/// A signer capable of producing signatures for a specific [`Curve`].
#[trait_variant::make(Send)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait Signer<C: Curve>: Sync {
    /// An error that can occur during [signing](Self::sign).
    type Error: Debug + Display;

    /// Get the public key of the signer.
    fn public_key(&self) -> C::PublicKey;

    /// Sign a given message and return a signature.
    ///
    /// NOTE: implementations MAY require `msg` to be prehash (i.e. output
    /// of cryptographic hash function) of a fixed length and return
    /// an error otherwise. Check corresponding docs before using.
    async fn sign(&self, msg: &[u8]) -> Result<C::Signature, Self::Error>;
}

/// A [`Signer`] that can produce [recoverable](RecoverableCurve::recover)
/// signatures.
#[trait_variant::make(Send)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait RecoverableSigner<C: RecoverableCurve>: Signer<C> {
    /// Sign a given message and return a signature along with recovery id.
    ///
    /// NOTE: implementations MAY require `msg` to be prehash (i.e. output
    /// of cryptographic hash function) of a fixed length and return
    /// an error otherwise. Check corresponding docs before using.
    async fn sign_recoverable(
        &self,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error>;
}

/// A helper trait that provides convenient-to-use methods on [`Signer`].
pub trait SignerExt {
    /// Get the public key of the signer.
    ///
    /// Forwards to [`Signer::public_key()`].
    #[inline]
    fn public_key<C: Curve>(&self) -> C::PublicKey
    where
        Self: Signer<C>,
    {
        Signer::<C>::public_key(self)
    }

    /// Sign a given message and return a signature.
    ///
    /// NOTE: implementations MAY require `msg` to be prehash (i.e. output
    /// of cryptographic hash function) of a fixed length and return
    /// an error otherwise. Check corresponding docs before using.
    ///
    /// This forwards to [`Signer::sign()`].
    // TODO: examples
    #[inline]
    fn sign<C: Curve>(
        &self,
        msg: &[u8],
    ) -> impl Future<Output = Result<C::Signature, Self::Error>> + Send
    where
        Self: Signer<C>,
    {
        Signer::<C>::sign(self, msg)
    }

    /// Sign a given message and return a signature along with recovery id.
    ///
    /// NOTE: implementations MAY require `msg` to be prehash (i.e. output
    /// of cryptographic hash function) of a fixed length and return
    /// an error otherwise. Check corresponding docs before using.
    ///
    /// This forwards to [`RecoverableSigner::sign_recoverable()`].
    #[inline]
    fn sign_recoverable<C: RecoverableCurve>(
        &self,
        msg: &[u8],
    ) -> impl Future<Output = Result<(C::Signature, C::RecoveryId), Self::Error>> + Send
    where
        Self: RecoverableSigner<C>,
    {
        RecoverableSigner::<C>::sign_recoverable(self, msg)
    }

    /// Cache [`.public_key()`](Signer::public_key) on a first access.
    ///
    /// This might be useful if a signer performs some expensive key derivation
    /// underneath.
    #[inline]
    fn cache_public_key<C: Curve>(self) -> CachePublicKey<C, Self>
    where
        Self: Signer<C> + Sized,
    {
        CachePublicKey::new(self)
    }
}
impl<S> SignerExt for S {}

/// An adaptor for [`Signer`] that caches the [`.public_key`](Signer::public_key)
/// on a first access.
///
/// See [`.cache_public_key()`](SignerExt::cache_public_key) for usages.
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

impl<C, S> Signer<C> for CachePublicKey<C, S>
where
    C: Curve,
    C::PublicKey: Clone + Send + Sync,
    S: Signer<C>,
{
    type Error = S::Error;

    #[inline]
    fn public_key(&self) -> C::PublicKey {
        self.public_key
            .get_or_init(|| self.signer.public_key())
            .clone()
    }

    #[inline]
    fn sign(&self, msg: &[u8]) -> impl Future<Output = Result<C::Signature, Self::Error>> + Send {
        self.signer.sign(msg)
    }
}

impl<C, S> RecoverableSigner<C> for CachePublicKey<C, S>
where
    C: RecoverableCurve,
    C::PublicKey: Clone + Send + Sync,
    S: RecoverableSigner<C>,
{
    #[inline]
    fn sign_recoverable(
        &self,
        msg: &[u8],
    ) -> impl Future<
        Output = Result<(<C>::Signature, <C as RecoverableCurve>::RecoveryId), Self::Error>,
    > + Send {
        self.signer.sign_recoverable(msg)
    }
}

/// Test helpers
#[cfg(test)]
#[allow(dead_code, clippy::redundant_pub_crate)]
pub(crate) mod tests {
    use std::fmt::Debug;

    use super::*;

    pub async fn test_sign_verify<C: Curve, S: Signer<C>>(signer: S, msg: impl AsRef<[u8]>) {
        let msg = msg.as_ref();
        let signature = signer.sign(msg).await.unwrap();
        assert!(
            C::verify(&signer.public_key(), msg, &signature),
            "signer produced invalid signature"
        );
    }

    pub async fn test_sign_recover<C, S>(signer: S, msg: impl AsRef<[u8]>)
    where
        C: RecoverableCurve<PublicKey: PartialEq + Debug>,
        S: RecoverableSigner<C>,
    {
        let msg = msg.as_ref();
        let (signature, recovery_id) = signer.sign_recoverable(msg).await.unwrap();

        assert_eq!(
            C::recover(msg, &signature, recovery_id),
            Some(signer.public_key()),
            "can't recover signer's public key"
        );
    }
}
