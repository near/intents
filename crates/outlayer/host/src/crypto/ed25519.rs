use defuse_kdf::{DeriveSigner, Ed25519};

use crate::{
    Host, HostView, bindings::outlayer::crypto::ed25519::Host as HostTrait, crypto::AppSigner,
};

impl<S> HostTrait for AppSigner<S>
where
    S: DeriveSigner<Ed25519, String>,
{
    fn derive_public_key(&mut self, path: String) -> wasmtime::Result<Vec<u8>> {
        Ok(DeriveSigner::<Ed25519, _>::derive_public_key(&self, path)
            .to_bytes()
            .to_vec())
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> wasmtime::Result<Vec<u8>> {
        Ok(DeriveSigner::<Ed25519, _>::derive_sign(&self, path, &msg).to_vec())
    }
}

impl HostTrait for Host<'_> {
    fn derive_public_key(&mut self, path: String) -> wasmtime::Result<Vec<u8>> {
        HostTrait::derive_public_key(&mut self.ctx().app_signer(), path)
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> wasmtime::Result<Vec<u8>> {
        HostTrait::sign(&mut self.ctx().app_signer(), path, msg)
    }
}
