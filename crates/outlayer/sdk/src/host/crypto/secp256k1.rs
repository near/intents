#[cfg(not(target_family = "wasm"))]
use crate::host::mock;
#[cfg(not(target_family = "wasm"))]
use defuse_outlayer_host::crypto::Secp256k1Host;

#[cfg(target_family = "wasm")]
use defuse_outlayer_sys as sys;

/// Secp256k1 (a.k.a. k256) public key in SEC-1 uncompressed form
/// **without** leading tag byte (0x04)
pub type PublicKey = [u8; 64];

/// Secp256k1 (a.k.a k256) signature encoded as concatenated `r`, `s` and
/// `v` (recovery byte)
pub type Signature = [u8; 65];

/// Derive public key from root for given application-specific path.
///
/// The derivation is **non-hierarchical** (or "plain"): derived
/// keys **do not** form a tree-like structure. Instead, child keys
/// are all derived from a single root key and can be considered as
/// "peers" to each other.
///
/// Returns secp256k1 public key encoded in SEC-1 uncompressed form
/// **without** leading tag byte (0x04).
// #[track_caller]
pub fn derive_public_key(path: impl AsRef<str>) -> PublicKey {
    let path = path.as_ref();

    #[cfg(target_family = "wasm")]
    let raw = sys::crypto::secp256k1::derive_public_key(path.as_ref());

    #[cfg(not(target_family = "wasm"))]
    let raw = mock::HOST
        .with_borrow_mut(|h| h.secp256k1_derive_public_key(path.to_string()))
        .expect("host");

    raw.try_into().expect("invalid length")
}

/// Sign 32-byte `prehash` with a secret key **internally** derived for
/// given application-specific `path`.
/// Prehash MUST be an output of a cryptographic hash function.
///
/// Returns a signature as concatenated `r`, `s` and `v` (recovery byte).
///
/// NOTE: signatures are non-deterministic, i.e. host implementation MAY
/// return different signatures for the same `path` and `prehash`.
// #[track_caller]
pub fn sign(path: impl AsRef<str>, prehash: &[u8; 32]) -> Signature {
    let path = path.as_ref();

    #[cfg(target_family = "wasm")]
    let raw = sys::crypto::secp256k1::sign(path, prehash.as_ref());

    #[cfg(not(target_family = "wasm"))]
    let raw = mock::HOST
        .with_borrow_mut(|h| h.secp256k1_sign(path.to_string(), prehash.as_ref().to_vec()))
        .expect("host");

    raw.try_into().expect("invalid length")
}
