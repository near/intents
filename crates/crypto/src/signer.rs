use crate::{Curve, RecoverableCurve};

pub trait Signer<C: Curve> {
    type Error;

    fn public_key(&self) -> C::PublicKey;

    async fn sign(&self, msg: &[u8]) -> Result<C::Signature, Self::Error>;
}

pub trait RecoverableSigner<C: RecoverableCurve>: Signer<C> {
    async fn sign_recoverable(
        &self,
        msg: &[u8],
    ) -> Result<(C::Signature, C::RecoveryId), Self::Error>;
}
