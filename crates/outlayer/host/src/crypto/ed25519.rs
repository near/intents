use defuse_outlayer_crypto::{DeriveSigner, ed25519::Ed25519};

use crate::State;

impl crate::bindings::outlayer::crypto::ed25519::Host for State<'_> {
    fn derive_public_key(&mut self, path: String) -> wasmtime::Result<Vec<u8>> {
        let path = self.tweak(path);

        let derived_pk = DeriveSigner::<Ed25519>::derive_public_key(&self.signer, &path);

        Ok(derived_pk.to_bytes().to_vec())
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> wasmtime::Result<Vec<u8>> {
        let tweak = self.tweak(path);

        let signature = DeriveSigner::<Ed25519>::derive_sign(&self.signer, &tweak, &msg);

        Ok(signature.to_vec())
    }
}
