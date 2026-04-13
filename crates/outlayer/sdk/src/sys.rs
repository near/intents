pub use defuse_outlayer_host::{
    ed25519::{Ed25519Host, Ed25519PublicKey, Ed25519Signature},
    secp256k1::{Secp256k1Host, Secp256k1PublicKey, Secp256k1Signature},
};

use defuse_outlayer_sys::crypto::{ed25519, secp256k1};
pub struct SysHost;

impl Ed25519Host for SysHost {
    fn ed25519_derive_public_key(path: impl AsRef<str>) -> Ed25519PublicKey {
        ed25519::derive_public_key(path.as_ref())
            .try_into()
            .expect("public key must be 32 bytes")
    }

    fn ed25519_sign(path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Ed25519Signature {
        ed25519::sign(path.as_ref(), msg.as_ref())
            .try_into()
            .expect("signature must be 64 bytes")
    }
}

impl Secp256k1Host for SysHost {
    fn secp256k1_derive_public_key(path: impl AsRef<str>) -> Secp256k1PublicKey {
        secp256k1::derive_public_key(path.as_ref())
            .try_into()
            .expect("public key must be 64 bytes")
    }

    fn secp256k1_sign(path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Secp256k1Signature {
        secp256k1::sign(path.as_ref(), msg.as_ref())
            .try_into()
            .expect("signature must be 65 bytes")
    }
}
