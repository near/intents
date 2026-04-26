use defuse_outlayer_crypto::{
    DerivableCurve, DerivablePublicKey, DeriveSigner,
    ed25519::{self, Ed25519},
    secp256k1::{self, Secp256k1},
};
use defuse_outlayer_primitives::crypto::DerivationPath;

use crate::Host;

pub trait Signer:
    DeriveSigner<Ed25519, PublicKey = ed25519::VerifyingKey>
    + DeriveSigner<Secp256k1, PublicKey = secp256k1::PublicKey>
{
}
impl<T> Signer for T where
    T: DeriveSigner<Ed25519, PublicKey = ed25519::VerifyingKey>
        + DeriveSigner<Secp256k1, PublicKey = secp256k1::PublicKey>
{
}

impl crate::bindings::outlayer::crypto::ed25519::Host for Host {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        crate::crypto::DeriveSigner::<Ed25519>::public_key(&self.signer)
            .derive(Ed25519::tweak(
                DerivationPath {
                    app_id: self.ctx.app_id.as_ref(),
                    path: path.into(),
                }
                .hash(),
            ))
            .as_bytes()
            .to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        crate::crypto::DeriveSigner::<Ed25519>::sign(
            &self.signer,
            Ed25519::tweak(
                DerivationPath {
                    app_id: self.ctx.app_id.as_ref(),
                    path: path.into(),
                }
                .hash(),
            ),
            &msg,
        )
        .to_vec()
    }
}

impl crate::bindings::outlayer::crypto::secp256k1::Host for Host {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        crate::crypto::DeriveSigner::<Secp256k1>::public_key(&self.signer)
            .derive(Secp256k1::tweak(
                DerivationPath {
                    app_id: self.ctx.app_id.as_ref(),
                    path: path.into(),
                }
                .hash(),
            ))
            .to_sec1_bytes()
            .to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        let (signature, recovery_id) = crate::crypto::DeriveSigner::<Secp256k1>::sign(
            &self.signer,
            Secp256k1::tweak(
                DerivationPath {
                    app_id: self.ctx.app_id.as_ref(),
                    path: path.into(),
                }
                .hash(),
            ),
            &msg,
        );

        let mut sig = signature.to_vec();
        sig.push(recovery_id.to_byte());
        sig
    }
}
