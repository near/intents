use core::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use impl_tools::autoimpl;

use crate::{Curve, RecoverableCurve};

/// A signer capable of producing signatures for a specific [`Curve`].
#[async_trait]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, Box<T>, Arc<T>)]
pub trait Signer<C: Curve> {
    /// An error that can occur during [signing](Self::sign).
    // TODO: trait bounds? StdError?
    type Error: Debug;

    /// Public key of the signer
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
#[async_trait]
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

/// Test helpers
#[cfg(test)]
#[allow(dead_code)]
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
