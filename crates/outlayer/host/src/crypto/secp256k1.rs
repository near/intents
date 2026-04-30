use anyhow::{Result, anyhow};
use defuse_outlayer_crypto::{DeriveSigner, secp256k1::Secp256k1};

use crate::State;

/// Trait defining secp256k1-related host functions available to the component
pub trait Secp256k1Host: Send {
    fn secp256k1_derive_public_key(&mut self, path: String) -> Result<Vec<u8>>;
    fn secp256k1_sign(&mut self, path: String, msg: Vec<u8>) -> Result<Vec<u8>>;
}

impl<T: Secp256k1Host + ?Sized> Secp256k1Host for &mut T {
    fn secp256k1_derive_public_key(&mut self, path: String) -> Result<Vec<u8>> {
        (**self).secp256k1_derive_public_key(path)
    }
    fn secp256k1_sign(&mut self, path: String, msg: Vec<u8>) -> Result<Vec<u8>> {
        (**self).secp256k1_sign(path, msg)
    }
}

impl Secp256k1Host for State<'_> {
    fn secp256k1_derive_public_key(&mut self, path: String) -> Result<Vec<u8>> {
        let path = self.tweak(path);

        let derived_pk = DeriveSigner::<Secp256k1>::derive_public_key(&self.signer, &path);

        Ok(derived_pk
            .to_encoded_point(false) // uncompressed
            .as_bytes()[1..] // trim leading SEC1 tag byte (0x04)
            .to_vec())
    }

    fn secp256k1_sign(&mut self, path: String, msg: Vec<u8>) -> Result<Vec<u8>> {
        let prehash: [u8; 32] = msg
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
