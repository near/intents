use defuse_outlayer_host::crypto::secp256k1::*;

pub fn derive_public_key(path: impl AsRef<str>) -> Secp256k1PublicKey {
    #[cfg(target_family = "wasm")]
    return ::defuse_outlayer_sys::crypto::secp256k1::derive_public_key(path.as_ref())
        .try_into()
        .expect("secp256k1 public key must be 64 bytes");

    #[cfg(not(target_family = "wasm"))]
    return crate::host::mock::HOST.with_borrow(|h| h.secp256k1_derive_public_key(path));
}

pub fn sign(path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Secp256k1Signature {
    #[cfg(target_family = "wasm")]
    return ::defuse_outlayer_sys::crypto::ed25519::sign(path.as_ref(), msg.as_ref())
        .try_into()
        .expect("secp256k1 signature must be 65 bytes");

    #[cfg(not(target_family = "wasm"))]
    return crate::host::mock::HOST.with_borrow(|h| h.secp256k1_sign(path, msg));
}
