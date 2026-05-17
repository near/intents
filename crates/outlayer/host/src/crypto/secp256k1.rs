use anyhow::anyhow;
use defuse_kdf::{DeriveSigner, Secp256k1};

use crate::{
    Host, HostView, bindings::outlayer::crypto::secp256k1::Host as HostTrait, crypto::AppSigner,
};

impl<S> HostTrait for AppSigner<S>
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

impl HostTrait for Host<'_> {
    fn derive_public_key(&mut self, path: String) -> wasmtime::Result<Vec<u8>> {
        HostTrait::derive_public_key(&mut self.ctx().app_signer(), path)
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> wasmtime::Result<Vec<u8>> {
        HostTrait::sign(&mut self.ctx().app_signer(), path, msg)
    }
}
