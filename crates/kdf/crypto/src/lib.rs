// TODO: merge with `defuse-crypto`

mod curve;
#[cfg(feature = "ed25519")]
pub mod ed25519;
// TODO: pub mods?
#[cfg(feature = "p256")]
pub mod p256;
#[cfg(feature = "secp256k1")]
pub mod secp256k1;

pub use self::curve::*;
