#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "secp256k1")]
pub mod secp256k1;
#[cfg(any(feature = "ed25519", feature = "secp256k1"))]
mod signer;

#[cfg(any(feature = "ed25519", feature = "secp256k1"))]
pub use self::signer::*;

pub use defuse_kdf::{self as kdf, DerivableCurve, Schema};
