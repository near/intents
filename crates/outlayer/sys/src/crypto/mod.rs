#[cfg(feature = "ed25519")]
mod ed25519_host;
#[cfg(feature = "secp256k1")]
mod secp256k1_host;

#[cfg(feature = "ed25519")]
pub use ed25519_host::ed25519;
#[cfg(feature = "secp256k1")]
pub use secp256k1_host::secp256k1;
