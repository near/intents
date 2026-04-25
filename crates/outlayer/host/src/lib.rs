pub use defuse_outlayer_crypto as crypto;
use defuse_outlayer_crypto::{
    DerivablePublicKey, ed25519::Ed25519, secp256k1::Secp256k1, signer::InMemorySigner,
};
use defuse_outlayer_primitives::{AppId, crypto::DerivationPath};

wasmtime::component::bindgen!({
    path: "../wit",
    world: "imports",
    imports: {
        // default: async | trappable,
    },
    ownership: Borrowing {
        duplicate_if_necessary: true
    },
});

pub struct HostContext<'a> {
    app_id: AppId<'a>,
    // signer: Box<dyn Signer>,
    signer: InMemorySigner,
}

impl<'a> HostContext<'a> {
    pub fn new(app_id: AppId<'a>, signer: InMemorySigner) -> Self {
        Self { app_id, signer }
    }

    pub fn with_app_id(&mut self, app_id: AppId<'a>) -> &mut Self {
        self.app_id = app_id;
        self
    }
}

impl outlayer::crypto::ed25519::Host for HostContext<'_> {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        crypto::DerivableSigningKey::<Ed25519>::public_key(&self.signer)
            .derive_from_borsh(DerivationPath {
                app_id: self.app_id.as_ref(),
                path: &path,
            })
            .as_bytes()
            .to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        crypto::DerivableSigningKey::<Ed25519>::sign_derive_from_borsh(
            &self.signer,
            DerivationPath {
                app_id: self.app_id.as_ref(),
                path: &path,
            },
            &msg,
        )
        .to_vec()
    }
}

impl outlayer::crypto::secp256k1::Host for HostContext<'_> {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        crypto::DerivableSigningKey::<Secp256k1>::public_key(&self.signer)
            .derive_from_borsh(DerivationPath {
                app_id: self.app_id.as_ref(),
                path: &path,
            })
            .to_sec1_bytes()
            .to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        let (signature, recovery_id) =
            crypto::DerivableSigningKey::<Secp256k1>::sign_derive_from_borsh(
                &self.signer,
                DerivationPath {
                    app_id: self.app_id.as_ref(),
                    path: &path,
                },
                &msg,
            );

        let mut sig = signature.to_vec();
        sig.push(recovery_id.to_byte());
        sig
    }
}
