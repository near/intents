use crate::SysHost;
use outlayer::host::ed25519;

pub use defuse_outlayer_host::ed25519::{Ed25519Host, Ed25519PublicKey, Ed25519Signature};

// TODO: may be make symlink?
wit_bindgen::generate!({
    path: "../wit",
    world: "ed25519-world",
});

impl Ed25519Host for SysHost {
    fn ed25519_get_project_public_key() -> Ed25519PublicKey {
        ed25519::get_project_public_key()
            .expect("failed to get project public key")
            .bytes
            .try_into()
            .expect("public key must be 32 bytes")
    }

    fn ed25519_derive_public_key(path: impl AsRef<str>) -> Ed25519PublicKey {
        ed25519::derive_public_key(path.as_ref())
            .expect("failed to derive public key")
            .bytes
            .try_into()
            .expect("public key must be 32 bytes")
    }

    fn ed25519_sign(path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Ed25519Signature {
        ed25519::sign(path.as_ref(), msg.as_ref())
            .expect("failed to sign message")
            .bytes
            .try_into()
            .expect("signature must be 64 bytes")
    }
}
