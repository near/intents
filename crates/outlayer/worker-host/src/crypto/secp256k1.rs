use defuse_outlayer_host::crypto::secp256k1::{
    Secp256k1Host, Secp256k1PublicKey, Secp256k1Signature,
};

use crate::WorkerHost;

impl Secp256k1Host for WorkerHost {
    fn secp256k1_derive_public_key(&self, _path: &str) -> Secp256k1PublicKey {
        unimplemented!()
    }

    fn secp256k1_sign(&self, _path: &str, _msg: &[u8]) -> Secp256k1Signature {
        unimplemented!()
    }
}
