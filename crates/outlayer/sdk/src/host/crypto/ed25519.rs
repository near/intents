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
#[track_caller]
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
#[track_caller]
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

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        "",
        hex!("c3b8c166a868349c1bbc348b710770795eaea6b57b7a23908b5e4046a6a2d528"),
    )]
    #[case(
        "test",
        hex!("6be812bfb9a103b335db64d736c8de1a32d4736e4a5907438013673e4c426776"),
    )]
    #[case(
        "0000000000000000000000000000000000000000000000000000000000000000",
        hex!("934ca7d08bc143c5cddeff0bdf1394d77350ae3c0b94465250c27326b826794b"),
    )]
    #[case(
        ".",
        hex!("673cb3243d8b905844ddd5c7a9f37e4e006c1e0e667c31bf9e7daa186cac8422"),
    )]
    fn derived_pk_has_not_changed(#[case] path: &str, #[case] expected: PublicKey) {
        println!("{}", hex::encode(derive_public_key(path)));
        assert_eq!(derive_public_key(path), expected);
    }
}
