// TODO: merge with `defuse-crypto`

mod curve;
#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "secp256k1")]
mod secp256k1;

pub use self::curve::*;
#[cfg(feature = "ed25519")]
pub use self::ed25519::*;
#[cfg(feature = "secp256k1")]
pub use self::secp256k1::*;

// re-exports
#[cfg(feature = "ed25519")]
pub use ed25519_dalek;
#[cfg(feature = "secp256k1")]
pub use k256;
