use crate::{WorkerHost, crypto::CryptoHost};
use defuse_outlayer_host::crypto::secp256k1::{
    Secp256k1Host, Secp256k1PublicKey, Secp256k1Signature,
};

pub struct WorkerSecp256k1Host;

impl Secp256k1Host for WorkerSecp256k1Host {
    fn secp256k1_derive_public_key(&self, _path: impl AsRef<str>) -> Secp256k1PublicKey {
        unimplemented!("secp256k1_derive_public_key is not implemented for WorkerHost");
    }

    fn secp256k1_sign(&self, _path: impl AsRef<str>, _msg: impl AsRef<[u8]>) -> Secp256k1Signature {
        unimplemented!("secp256k1_sign is not implemented for WorkerHost");
    }
}

impl Secp256k1Host for CryptoHost {
    fn secp256k1_derive_public_key(&self, path: impl AsRef<str>) -> Secp256k1PublicKey {
        self.secp256k1.secp256k1_derive_public_key(path)
    }

    fn secp256k1_sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Secp256k1Signature {
        self.secp256k1.secp256k1_sign(path, msg)
    }
}

impl Secp256k1Host for WorkerHost {
    fn secp256k1_derive_public_key(&self, path: impl AsRef<str>) -> Secp256k1PublicKey {
        self.crypto.secp256k1_derive_public_key(path)
    }

    fn secp256k1_sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Secp256k1Signature {
        self.crypto.secp256k1_sign(path, msg)
    }
}
