#[cfg(not(target_family = "wasm"))]
use crate::host::mock;
#[cfg(not(target_family = "wasm"))]
use defuse_outlayer_host::bindings::outlayer::crypto::secp256k1::Host;

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
#[track_caller]
pub fn derive_public_key(path: impl AsRef<str>) -> PublicKey {
    let path = path.as_ref();

    #[cfg(target_family = "wasm")]
    let raw = sys::crypto::secp256k1::derive_public_key(path.as_ref());

    #[cfg(not(target_family = "wasm"))]
    let raw = mock::HOST
        .with_borrow_mut(|h| h.derive_public_key(path.to_string()))
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
#[track_caller]
pub fn sign(path: impl AsRef<str>, prehash: &[u8; 32]) -> Signature {
    let path = path.as_ref();

    #[cfg(target_family = "wasm")]
    let raw = sys::crypto::secp256k1::sign(path, prehash.as_ref());

    #[cfg(not(target_family = "wasm"))]
    let raw = mock::HOST
        .with_borrow_mut(|h| h.sign(path.to_string(), prehash.as_ref().to_vec()))
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
        hex!("4511f00e3e70a9f3ccacb58af0dae770aca04670c30bcf3de93a93c8d967d5fbf13b788f4f8654bbb81f629c4b9d0d8b8070979d004ea0e27b9dbad675a99675"),
    )]
    #[case(
        "test",
        hex!("9c2567a8bdc4a5728050198d0fc991c53ebc375de0fa74c36f46a6d8d3361c400796972228d59351380c7c7bc221381ed4ea93af7a126a8bed8dfa0b0821cbaf"),
    )]
    #[case(
        "0000000000000000000000000000000000000000000000000000000000000000",
        hex!("f024688b06bb178284e3249ecdf6fb962371bb4f3fb690f7030b6303a4954e1df0744e6747a5fa3e2ffd04eb9d5198b410a4cf1e4c93df30c53018110eee0306"),
    )]
    #[case(
        ".",
        hex!("a55d576618eaac125e3d747494ada0b1534d0ea222d0d99806b750686c24ae26d2f15d2c471d91c300e844c9e6d893a2b853f9ea8da359e7d67811f0001e3c75"),
    )]
    fn derived_pk_has_not_changed(#[case] path: &str, #[case] expected: PublicKey) {
        assert_eq!(derive_public_key(path), expected);
    }
}
