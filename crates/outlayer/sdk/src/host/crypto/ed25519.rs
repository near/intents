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
// #[track_caller]
pub fn derive_public_key(path: impl AsRef<str>) -> PublicKey {
    let path = path.as_ref();

    #[cfg(target_family = "wasm")]
    let raw = sys::crypto::ed25519::derive_public_key(path);

    #[cfg(not(target_family = "wasm"))]
    let raw = mock::HOST
        .with_borrow_mut(|h| h.derive_public_key(path.to_string()))
        .expect("host");

    raw.try_into().expect("invalid length")
}

/// Sign given message with a secret key **internally** derived for
/// given application-specific path.
///
/// NOTE: signatures are non-deterministic, i.e. host implementation MAY
/// return different signatures for the same `path` and `msg`.
// #[track_caller]
pub fn sign(path: impl AsRef<str>, msg: impl AsRef<[u8]>) -> Signature {
    let path = path.as_ref();
    let msg = msg.as_ref();

    #[cfg(target_family = "wasm")]
    let raw = sys::crypto::ed25519::sign(path, msg);

    #[cfg(not(target_family = "wasm"))]
    let raw = mock::HOST
        .with_borrow_mut(|h| h.sign(path.to_string(), msg.to_vec()))
        .expect("host");

    raw.try_into().expect("invalid length")
}
