// TODO: sign()?
pub use defuse_kdf_crypto::*;

pub trait SignatureScheme<M> {
    type Curve: VerifiableCurve<Self::VerifiableMessage>;

    // TODO: schema?

    type VerifiableMessage;

    type Signature;

    fn check_prepare(
        &self,
        msg: M,
        signature: &Self::Signature,
    ) -> Option<(Self::VerifiableMessage, <Self::Curve as Curve>::Signature)>;

    fn verify(
        &self,
        public_key: &<Self::Curve as Curve>::PublicKey,
        msg: M,
        signature: &Self::Signature,
    ) -> bool {
        let Some((msg, signature)) = self.check_prepare(msg, signature) else {
            return false;
        };
        Self::Curve::verify(public_key, msg, &signature)
    }

    // TODO: how to pass UserVerification?
    // associated const? or embed in PublicKey? - bad idea
    // via &self?
    // fn verify(public_key: &Self::PublicKey, msg: M, signature: &Self::Signature) -> bool;
}

pub trait RecoverableSignatureScheme<M>: SignatureScheme<M>
where
    Self: SignatureScheme<M>,
    Self::Curve: RecoverableCurve<Self::VerifiableMessage>,
{
    type RecoverableSignature;

    #[allow(clippy::type_complexity)] // TODO: remove
    fn check_prepare_recoverable(
        &self,
        msg: M,
        signature: &Self::RecoverableSignature,
    ) -> Option<(
        Self::VerifiableMessage,
        <Self::Curve as Curve>::Signature,
        <Self::Curve as RecoverableCurve<Self::VerifiableMessage>>::RecoveryId,
    )>;

    fn recover(
        &self,
        msg: M,
        signature: &Self::RecoverableSignature,
    ) -> Option<<Self::Curve as Curve>::PublicKey> {
        let (msg, signature, recovery_id) = self.check_prepare_recoverable(msg, signature)?;

        Self::Curve::recover(msg, &signature, recovery_id)
    }
}
