use anyhow::anyhow;
use defuse_kdf::{DeriveSigner, secp256k1::Secp256k1};

use crate::crypto::AppSigner;

impl<S> crate::bindings::outlayer::crypto::secp256k1::Host for AppSigner<S>
where
    S: DeriveSigner<Secp256k1, String>,
{
    fn derive_public_key(&mut self, path: String) -> wasmtime::Result<Vec<u8>> {
        Ok(DeriveSigner::<Secp256k1, _>::derive_public_key(&self, path)
            .to_encoded_point(false) // uncompressed
            .as_bytes()[1..] // trim leading SEC1 tag byte (0x04)
            .to_vec())
    }

    fn sign(&mut self, path: String, prehash: Vec<u8>) -> wasmtime::Result<Vec<u8>> {
        let prehash: [u8; 32] = prehash
            .try_into()
            .map_err(|v: Vec<_>| anyhow!("prehash must be 32 bytes long, got: {}", v.len()))?;

        let (signature, recovery_id) =
            DeriveSigner::<Secp256k1, _>::derive_sign(&self, path, &prehash);

        let mut sig = signature.to_vec();
        sig.push(recovery_id.to_byte());
        Ok(sig)
    }
}
