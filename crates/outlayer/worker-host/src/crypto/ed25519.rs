use crate::{WorkerHost, crypto::CryptoHost};
use defuse_outlayer_host::crypto::ed25519::{Ed25519Host, Ed25519PublicKey, Ed25519Signature};

#[derive(Debug, Default)]
pub struct WorkerEd25519Host;

impl Ed25519Host for WorkerEd25519Host {
    fn ed25519_derive_public_key(&self, _path: impl AsRef<str>) -> Ed25519PublicKey {
        unimplemented!("ed25519_derive_public_key is not implemented for WorkerHost");
    }

    fn ed25519_sign(&self, _path: impl AsRef<str>, _msg: impl AsRef<[u8]>) -> Ed25519Signature {
        unimplemented!("ed25519_sign is not implemented for WorkerHost");
    }
}

impl Ed25519Host for CryptoHost {
    fn ed25519_derive_public_key(&self, path: impl AsRef<str>) -> Ed25519PublicKey {
        self.ed25519.ed25519_derive_public_key(path)
    }

    fn ed25519_sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Ed25519Signature {
        self.ed25519.ed25519_sign(path, msg)
    }
}

impl Ed25519Host for WorkerHost {
    fn ed25519_derive_public_key(&self, path: impl AsRef<str>) -> Ed25519PublicKey {
        self.crypto.ed25519_derive_public_key(path)
    }

    fn ed25519_sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Ed25519Signature {
        self.crypto.ed25519_sign(path, msg)
    }
}
