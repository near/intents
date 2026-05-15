use defuse_outlayer_kdf_app::{DeriveSigner, WithAppId, kdf::ed25519::Ed25519};

impl<S> crate::bindings::outlayer::crypto::ed25519::Host for WithAppId<'_, S>
where
    Self: DeriveSigner<Ed25519, String>,
{
    fn derive_public_key(&mut self, path: String) -> wasmtime::Result<Vec<u8>> {
        Ok(DeriveSigner::derive_public_key(&self, path)
            .to_bytes()
            .to_vec())
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> wasmtime::Result<Vec<u8>> {
        Ok(DeriveSigner::derive_sign(&self, path, &msg).to_vec())
    }
}
