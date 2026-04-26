use defuse_outlayer_crypto::{DeriveSigner, ed25519::Ed25519};

use crate::Host;

impl crate::bindings::outlayer::crypto::ed25519::Host for Host {
    fn derive_public_key(&mut self, path: String) -> Vec<u8> {
        let tweak = self.tweak::<Ed25519>(path);

        let derived_pk = DeriveSigner::<Ed25519>::derive_public_key(&self.signer, &tweak);

        derived_pk.to_bytes().to_vec()
    }

    fn sign(&mut self, path: String, msg: Vec<u8>) -> Vec<u8> {
        let tweak = self.tweak::<Ed25519>(path);

        let signature = crate::crypto::DeriveSigner::<Ed25519>::sign(&self.signer, &tweak, &msg);

        signature.to_vec()
    }
}
