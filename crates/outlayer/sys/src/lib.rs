#[cfg(not(any(feature = "ed25519", feature = "secp256k1")))]
compile_error!(
    r#"At least one of these features should be enabled:
- "ed25519"
- "secp256k1"
"#
);

pub mod crypto;
