#[cfg(not(any(feature = "ed25519", feature = "secp256k1")))]
compile_error!(
    r#"At least one of these features should be enabled:
- "ed25519"
- "secp256k1"
"#
);

#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "secp256k1")]
mod secp256k1;

pub mod crypto {
    #[cfg(feature = "ed25519")]
    pub use crate::ed25519::ed25519;
    #[cfg(feature = "secp256k1")]
    pub use crate::secp256k1::secp256k1;
}
