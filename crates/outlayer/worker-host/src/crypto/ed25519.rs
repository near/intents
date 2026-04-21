use defuse_outlayer_host::crypto::ed25519::{Ed25519Host, Ed25519PublicKey, Ed25519Signature};

use crate::WorkerHost;

impl Ed25519Host for WorkerHost {
    fn ed25519_derive_public_key(&self, _path: &str) -> Ed25519PublicKey {
        unimplemented!()
    }

    fn ed25519_sign(&self, _path: &str, _msg: &[u8]) -> Ed25519Signature {
        unimplemented!()
    }
}
