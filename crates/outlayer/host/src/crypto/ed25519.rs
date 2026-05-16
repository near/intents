use defuse_kdf::{DeriveSigner, ed25519::Ed25519};

use crate::crypto::AppSigner;

impl<S> crate::bindings::outlayer::crypto::ed25519::Host for AppSigner<S>
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
