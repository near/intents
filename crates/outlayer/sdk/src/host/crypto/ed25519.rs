#[cfg(not(target_family = "wasm"))]
use crate::host::mock;
#[cfg(not(target_family = "wasm"))]
use defuse_outlayer_host::bindings::outlayer::crypto::ed25519::Host;

#[cfg(target_family = "wasm")]
use defuse_outlayer_sys as sys;

/// Ed25519 public key
pub type PublicKey = [u8; 32];
/// Ed25519 signature
pub type Signature = [u8; 64];

/// Derive public key from root for given application-specific path.
///
/// The derivation is **non-hierarchical** (or "plain"): derived
/// keys **do not** form a tree-like structure. Instead, child keys
/// are all derived from a single root key and can be considered as
/// "peers" to each other.
pub fn derive_public_key(path: impl AsRef<str>) -> PublicKey {
    #[cfg(target_family = "wasm")]
    {
        sys::crypto::ed25519::derive_public_key(path.as_ref())
    }
    #[cfg(not(target_family = "wasm"))]
    {
        mock::HOST
            .with_borrow_mut(|h| h.derive_public_key(path.as_ref().to_string()))
            .expect("host")
    }
    .try_into()
    .expect("invalid length")
}

/// Sign given message with a secret key **intetnally** derived for
/// given application-specific path.
pub fn sign(path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Signature {
    #[cfg(target_family = "wasm")]
    {
        sys::crypto::ed25519::sign(path.as_ref(), msg.as_ref())
    }
    #[cfg(not(target_family = "wasm"))]
    {
        mock::HOST
            .with_borrow_mut(|h| h.sign(path.as_ref().to_string(), msg.as_ref().to_vec()))
            .expect("host")
    }
    .try_into()
    .expect("invalid length")
}
