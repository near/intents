use anyhow::anyhow;
use defuse_outlayer_crypto::{DeriveSigner, secp256k1::Secp256k1};

use crate::Host;

impl crate::bindings::outlayer::crypto::secp256k1::Host for Host<'_> {
    fn derive_public_key(&mut self, path: String) -> wasmtime::Result<Vec<u8>> {
        let path = self.tweak(path);

        let derived_pk = DeriveSigner::<Secp256k1>::derive_public_key(&self.signer, &path);

        Ok(derived_pk
            .to_encoded_point(false) // uncompressed
            .as_bytes()[1..] // trim leading SEC1 tag byte (0x04)
            .to_vec())
    }

    fn sign(&mut self, path: String, prehash: Vec<u8>) -> wasmtime::Result<Vec<u8>> {
        let prehash: [u8; 32] = prehash
            .try_into()
            .map_err(|v: Vec<_>| anyhow!("prehash must be 32 bytes long, got: {}", v.len()))?;

        let path = self.tweak(path);

        let (signature, recovery_id) =
            DeriveSigner::<Secp256k1>::derive_sign(&self.signer, &path, &prehash);

        let mut sig = signature.to_vec();
        sig.push(recovery_id.to_byte());
        Ok(sig)
    }
}
