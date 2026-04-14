use defuse_outlayer_sys::crypto::secp256k1;
use defuse_outlayer_types::secp256k1::{Secp256k1Host, Secp256k1PublicKey, Secp256k1Signature};

use super::SysHost;

impl Secp256k1Host for SysHost {
    fn secp256k1_derive_public_key(&self, path: impl AsRef<str>) -> Secp256k1PublicKey {
        secp256k1::derive_public_key(path.as_ref())
            .try_into()
            .expect("public key must be 64 bytes")
    }

    fn secp256k1_sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Secp256k1Signature {
        secp256k1::sign(path.as_ref(), msg.as_ref())
            .try_into()
            .expect("signature must be 65 bytes")
    }
}
