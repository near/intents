use defuse_outlayer_host::ed25519::{Ed25519Host, Ed25519PublicKey, Ed25519Signature};
use defuse_outlayer_sys::crypto::ed25519;

use crate::SysHost;

impl Ed25519Host for SysHost {
    fn ed25519_derive_public_key(&self, path: impl AsRef<str>) -> Ed25519PublicKey {
        ed25519::derive_public_key(path.as_ref())
            .try_into()
            .expect("public key must be 32 bytes")
    }

    fn ed25519_sign(&self, path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Ed25519Signature {
        ed25519::sign(path.as_ref(), msg.as_ref())
            .try_into()
            .expect("signature must be 64 bytes")
    }
}
