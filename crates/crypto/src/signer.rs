use std::{
    fmt::{Debug, Display},
    sync::Arc,
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

/// An extention trait for [`Signer`].
pub trait SignerExt {
    /// Get the public key of the signer on a curve `C`.
    ///
    /// This method forwards to [`Signer::public_key()`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use defuse_crypto::{SignerExt, ed25519::Ed25519};
    /// # use defuse_crypto::ed25519::ed25519_dalek;
    /// # use hex_literal::hex;
    /// # let signer = ed25519_dalek::SigningKey::from_bytes(
    /// #     &hex!("5e20282e090a2be1487b29a38750d746ffcdb0b47ebff09441a2167b7523260f"),
    /// # );
    ///
    /// let public_key = signer.public_key::<Ed25519>();
    /// # assert_eq!(
    /// #     *public_key.as_bytes(),
    /// #     hex!("a0453cc99514600fd5f65a7c2e1e8c147fe1fdb3df7f7fe8d4bf18d619865110"),
    /// # );
    /// ```
    #[inline]
    fn public_key<C: Curve>(&self) -> C::PublicKey
    where
        Self: Signer<C>,
    {
        Signer::<C>::public_key(self)
    }

    /// Sign a given message and return a signature.
    ///
    /// **NOTE**: implementations MAY require `msg` to be prehash (i.e. output
    /// of cryptographic hash function) of a fixed length and return an error
    /// otherwise. Check corresponding docs before using.
    ///
    /// This method forwards to [`Signer::sign()`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use defuse_crypto::{Curve, SignerExt, ed25519::Ed25519};
    /// # use defuse_crypto::ed25519::ed25519_dalek;
    ///
    /// # #[tokio::main(flavor = "local")]
    /// # async fn main() -> Result<(), Box<dyn core::error::Error>> {
    /// # let signer = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
    /// let msg = b"message";
    /// let signature = signer.sign::<Ed25519>(msg).await?;
    ///
    /// assert!(Ed25519::verify(
    ///     &signer.public_key::<Ed25519>(),
    ///     msg,
    ///     &signature,
    /// ));
    /// # Ok(()) }
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
    /// **NOTE**: implementations MAY require `msg` to be prehash (i.e. output
    /// of cryptographic hash function) of a fixed length and return an error
    /// otherwise. Check corresponding docs before using.
    ///
    /// This method forwards to [`RecoverableSigner::sign_recoverable()`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use defuse_crypto::{RecoverableCurve, SignerExt, secp256k1::Secp256k1};
    /// # use defuse_crypto::secp256k1::k256;
    /// # use hex_literal::hex;
    ///
    /// # #[tokio::main(flavor = "local")]
    /// # async fn main() -> Result<(), Box<dyn core::error::Error>> {
    /// # fn make_signer() -> impl defuse_crypto::RecoverableSigner<Secp256k1, Error: core::error::Error> {
    /// #     k256::ecdsa::SigningKey::from_bytes(
    /// #         &hex!("f9151587531d664b5e266a1943c56530d8c6cebeedf631b15736176f1f9d8a90").into(),
    /// #     )
    /// #     .unwrap()
    /// # }
    /// # let signer = make_signer();
    /// // secp256k1 signs over 32-byte prehash
    /// let prehash = hex!("f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2");
    /// let (sig, recovery_id) = signer.sign_recoverable::<Secp256k1>(&prehash).await?;
    ///
    /// assert_eq!(
    ///     Secp256k1::recover(&prehash, &sig, recovery_id),
    ///     Some(signer.public_key::<Secp256k1>()),
    /// );
    /// # Ok(()) }
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

    /// Calculate [`.public_key()`](Signer::public_key) and cache its value for
    /// further invocations.
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

/// An adaptor for [`Signer`] that caches the [`.public_key`](Signer::public_key).
///
/// See [`.cache_public_key()`](SignerExt::cache_public_key) for usages.
#[autoimpl(Deref using self.signer)]
#[autoimpl(Debug, Clone, PartialEq, Eq where C::PublicKey: trait, S: trait)]
#[cfg_attr(
    feature = "zeroize",
    derive(zeroize::Zeroize),
    zeroize(bound = "S: zeroize::Zeroize")
)]
pub struct CachePublicKey<C: Curve, S: Signer<C>> {
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    public_key: C::PublicKey,
    signer: S,
}

impl<C, S> CachePublicKey<C, S>
where
    C: Curve,
    S: Signer<C>,
{
    #[inline]
    fn new(signer: S) -> Self {
        Self {
            public_key: signer.public_key(),
            signer,
        }
    }
}

/// Not `#[derive(ZeroizeOnDrop)]`: its `Drop` impl would need `S: Zeroize`,
/// which the struct itself doesn't require.
#[cfg(feature = "zeroize")]
impl<C, S> zeroize::ZeroizeOnDrop for CachePublicKey<C, S>
where
    C: Curve,
    S: Signer<C> + zeroize::ZeroizeOnDrop,
{
}

impl<C, S> Signer<C> for CachePublicKey<C, S>
where
    C: Curve<PublicKey: Clone + Send + Sync>,
    S: Signer<C>,
{
    type Error = S::Error;

    #[inline]
    fn public_key(&self) -> C::PublicKey {
        self.public_key.clone()
    }

    #[inline]
    fn sign(&self, msg: &[u8]) -> impl Future<Output = Result<C::Signature, Self::Error>> + Send {
        self.signer.sign(msg)
    }
}

impl<C, S> RecoverableSigner<C> for CachePublicKey<C, S>
where
    C: RecoverableCurve<PublicKey: Clone + Send + Sync>,
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
