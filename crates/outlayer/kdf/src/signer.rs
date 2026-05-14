use defuse_outlayer_crypto::{Curve, DerivableCurve};

use crate::DerivationScheme;

pub trait DeriveSigner<C, S, P>
where
    C: DerivableCurve + ?Sized,
    S: DerivationScheme<C, P> + ?Sized,
{
    fn public_key(&self) -> C::PublicKey;

    fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature;

    fn derive_public_key(&self, path: P) -> C::PublicKey {
        let master_pk = self.public_key();

        S::derive_public_key(&master_pk, path)
    }
}

// TODO: wrap as PrivateKey<T>
impl<S, T, P> DeriveSigner<T::Curve, S, P> for T
where
    T: defuse_outlayer_crypto::DeriveSigner,
    S: DerivationScheme<T::Curve, P> + ?Sized,
{
    fn public_key(&self) -> <T::Curve as Curve>::PublicKey {
        defuse_outlayer_crypto::DeriveSigner::public_key(self)
    }

    fn derive_sign(
        &self,
        path: P,
        msg: &<T::Curve as Curve>::Message,
    ) -> <T::Curve as Curve>::Signature {
        let tweak = S::tweak(path);

        defuse_outlayer_crypto::DeriveSigner::derive_sign(&self, &tweak, msg)
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncDeriveSigner<C, S, P>
where
    C: DerivableCurve + ?Sized,
    S: DerivationScheme<C, P> + ?Sized,
{
    fn public_key(&self) -> C::PublicKey;

    async fn derive_sign(&self, path: P, msg: &C::Message) -> C::Signature;

    fn derive_public_key(&self, path: P) -> C::PublicKey {
        let master_pk = self.public_key();

        S::derive_public_key(&master_pk, path)
    }
}
