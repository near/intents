use defuse_outlayer_crypto::{
    DerivableCurve, DerivablePublicKey,
    ed25519::{self, Ed25519},
    secp256k1::{self, Secp256k1},
};
use defuse_outlayer_primitives::{AppId, crypto::DerivationPath};

pub trait Signer:
    crate::crypto::DerivableSigningKey<Ed25519, PublicKey = ed25519::VerifyingKey>
    + crate::crypto::DerivableSigningKey<Secp256k1, PublicKey = secp256k1::PublicKey>
{
}
impl<T> Signer for T where
    T: crate::crypto::DerivableSigningKey<Ed25519, PublicKey = ed25519::VerifyingKey>
        + crate::crypto::DerivableSigningKey<Secp256k1, PublicKey = secp256k1::PublicKey>
{
}

pub struct HostContext {
    app_id: AppId<'static>,
    signer: Box<dyn Signer>,
}

impl HostContext {
    pub fn new(app_id: impl Into<AppId<'static>>, signer: impl Signer + 'static) -> Self {
        Self {
            app_id: app_id.into(),
            signer: Box::new(signer),
        }
    }

    pub fn with_app_id(&mut self, app_id: impl Into<AppId<'static>>) -> &mut Self {
        self.app_id = app_id.into();
        self
    }
}

impl crate::outlayer::crypto::ed25519::Host for HostContext {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        crate::crypto::DerivableSigningKey::<Ed25519>::public_key(&self.signer)
            .derive_from_borsh(DerivationPath {
                app_id: self.app_id.as_ref(),
                path: path.into(),
            })
            .as_bytes()
            .to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        crate::crypto::DerivableSigningKey::<Ed25519>::sign_derive_from_tweak(
            &self.signer,
            Ed25519::derive_tweak(DerivationPath {
                app_id: self.app_id.as_ref(),
                path: path.into(),
            }),
            &msg,
        )
        .to_vec()
    }
}

impl crate::outlayer::crypto::secp256k1::Host for HostContext {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        crate::crypto::DerivableSigningKey::<Secp256k1>::public_key(&self.signer)
            .derive_from_borsh(DerivationPath {
                app_id: self.app_id.as_ref(),
                path: path.into(),
            })
            .to_sec1_bytes()
            .to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        let (signature, recovery_id) =
            crate::crypto::DerivableSigningKey::<Secp256k1>::sign_derive_from_tweak(
                &self.signer,
                Secp256k1::derive_tweak(DerivationPath {
                    app_id: self.app_id.as_ref(),
                    path: path.into(),
                }),
                &msg,
            );

        let mut sig = signature.to_vec();
        sig.push(recovery_id.to_byte());
        sig
    }
}
