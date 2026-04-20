use defuse_outlayer_host::crypto::ed25519::{Ed25519Host, Ed25519PublicKey, Ed25519Signature};

pub fn derive_public_key(path: impl AsRef<str>) -> Ed25519PublicKey {
    #[cfg(target_family = "wasm")]
    return ::defuse_outlayer_sys::crypto::ed25519::derive_public_key(path.as_ref())
        .try_into()
        .expect("ed25519 public key must be 32 bytes");

    #[cfg(not(target_family = "wasm"))]
    return crate::host::mock::HOST.with_borrow(|h| h.ed25519_derive_public_key(path.as_ref()));
}

pub fn sign(path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Ed25519Signature {
    #[cfg(target_family = "wasm")]
    return ::defuse_outlayer_sys::crypto::ed25519::sign(path.as_ref(), msg.as_ref())
        .try_into()
        .expect("ed25519 signature must be 64 bytes");

    #[cfg(not(target_family = "wasm"))]
    return crate::host::mock::HOST.with_borrow(|h| h.ed25519_sign(path.as_ref(), msg.as_ref()));
}
