use defuse_digest::{Digest, sha3::Keccak256};
use defuse_kdf_crypto::{Curve, RecoverableCurve, Secp256k1};
use defuse_signature_scheme::{RecoverableSignatureScheme, SignatureScheme};

pub struct Erc191;

impl Erc191 {
    pub fn prehash(msg: impl AsRef<[u8]>) -> [u8; 32] {
        let msg = msg.as_ref();

        Keccak256::new_with_prefix(b"\x19Ethereum Signed Message:\n")
            .chain_update(msg.len().to_string())
            .chain_update(msg)
            .finalize()
            .into()
    }
}

impl<M> SignatureScheme<M> for Erc191
where
    M: AsRef<[u8]>,
{
    type Curve = Secp256k1;

    type VerifiableMessage = [u8; 32];

    type Signature = <Self::Curve as Curve>::Signature;

    fn check_prepare(
        &self,
        msg: M,
        signature: &Self::Signature,
    ) -> Option<(Self::VerifiableMessage, <Self::Curve as Curve>::Signature)> {
        Some((
            Self::prehash(msg),
            // TODO: avoid cloning
            *signature,
        ))
    }
}

impl<M> RecoverableSignatureScheme<M> for Erc191
where
    M: AsRef<[u8]>,
{
    type RecoverableSignature = (
        <Self::Curve as Curve>::Signature,
        <Self::Curve as RecoverableCurve<Self::VerifiableMessage>>::RecoveryId,
    );

    fn check_prepare_recoverable(
        &self,
        msg: M,
        (signature, recovery_id): &Self::RecoverableSignature,
    ) -> Option<(
        Self::VerifiableMessage,
        <Self::Curve as Curve>::Signature,
        <Self::Curve as RecoverableCurve<Self::VerifiableMessage>>::RecoveryId,
    )> {
        Some((
            Self::prehash(msg),
            // TODO: avoid cloning
            *signature,
            *recovery_id,
        ))
    }
}

#[cfg(test)]
mod tests {
    use defuse_kdf_crypto::k256::{
        EncodedPoint,
        ecdsa::{RecoveryId, Signature, VerifyingKey},
    };
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        hex!("85a66984273f338ce4ef7b85e5430b008307e8591bb7c1b980852cf6423770b801f41e9438155eb53a5e20f748640093bb42ae3aeca035f7b7fd7a1a21f22f68"),
        "Hello world!",
        hex!("7800a70d05cde2c49ed546a6ce887ce6027c2c268c0285f6efef0cdfc4366b23643790f67a86468ee8301ed12cfffcb07c6530f90a9327ec057800fabd332e4701"),
    )]
    fn verify_ok(
        #[case] public_key: [u8; 64],
        #[case] msg: impl AsRef<[u8]>,
        #[case] signature: [u8; 65],
    ) {
        let msg = msg.as_ref();
        let [signature @ .., v] = signature;
        let public_key = VerifyingKey::from_encoded_point(&EncodedPoint::from_untagged_bytes(
            &public_key.into(),
        ))
        .unwrap();
        let signature = Signature::from_bytes(&signature.into()).unwrap();
        let recovery_id = RecoveryId::from_byte(v).unwrap();

        assert!(Erc191.verify(&public_key, msg, &signature));

        assert_eq!(
            Erc191.recover(msg, &(signature, recovery_id)),
            Some(public_key)
        );
    }
}
