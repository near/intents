#[cfg(not(target_family = "wasm"))]
use defuse_outlayer_host as host;
#[cfg(target_family = "wasm")]
use defuse_outlayer_sys as sys;

pub type PublicKey = [u8; 32];
pub type Signature = [u8; 64];

pub fn derive_public_key(path: impl AsRef<str>) -> PublicKey {
    #[cfg(target_family = "wasm")]
    {
        sys::crypto::ed25519::derive_public_key(path.as_ref())
    }
    #[cfg(not(target_family = "wasm"))]
    {
        crate::host::mock::HOST.with_borrow_mut(|h| {
            host::outlayer::crypto::ed25519::Host::derive_public_key(
                h,
                path.as_ref().to_string(), // TODO
            )
        })
    }
    .try_into()
    .expect("invalid length")
}

pub fn sign(path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Signature {
    #[cfg(target_family = "wasm")]
    {
        sys::crypto::ed25519::sign(path.as_ref(), msg.as_ref())
    }
    #[cfg(not(target_family = "wasm"))]
    {
        crate::host::mock::HOST.with_borrow_mut(|h| {
            host::outlayer::crypto::ed25519::Host::sign(
                h,
                path.as_ref().to_string(), // TODO
                msg.as_ref().to_vec(),     // TODO
            )
        })
    }
    .try_into()
    .expect("invalid length")
}
