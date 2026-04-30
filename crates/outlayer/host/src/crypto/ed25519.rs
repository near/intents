use anyhow::Result;
use defuse_outlayer_crypto::{DeriveSigner, ed25519::Ed25519};

use crate::State;

/// Trait defining ed25519-related host functions available to the component
pub trait Ed25519Host: Send {
    fn ed25519_derive_public_key(&mut self, path: String) -> Result<Vec<u8>>;
    fn ed25519_sign(&mut self, path: String, msg: Vec<u8>) -> Result<Vec<u8>>;
}

impl<T: Ed25519Host + ?Sized> Ed25519Host for &mut T {
    fn ed25519_derive_public_key(&mut self, path: String) -> Result<Vec<u8>> {
        (**self).ed25519_derive_public_key(path)
    }
    fn ed25519_sign(&mut self, path: String, msg: Vec<u8>) -> Result<Vec<u8>> {
        (**self).ed25519_sign(path, msg)
    }
}

impl Ed25519Host for State<'_> {
    fn ed25519_derive_public_key(&mut self, path: String) -> Result<Vec<u8>> {
        let path = self.tweak(path);

        let derived_pk = DeriveSigner::<Ed25519>::derive_public_key(&self.signer, &path);

        Ok(derived_pk.to_bytes().to_vec())
    }

    fn ed25519_sign(&mut self, path: String, msg: Vec<u8>) -> Result<Vec<u8>> {
        let tweak = self.tweak(path);

        let signature = DeriveSigner::<Ed25519>::derive_sign(&self.signer, &tweak, &msg);

        Ok(signature.to_vec())
    }
}
