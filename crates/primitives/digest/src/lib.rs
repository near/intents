#[cfg(feature = "near-contract")]
mod near;

#[cfg(feature = "near-contract")]
pub use near::*;

#[cfg(all(feature = "sha2", not(feature = "near-contract")))]
pub use sha2::Sha256;

#[cfg(all(feature = "sha3", not(feature = "near-contract")))]
pub use sha3::Keccak256;

pub use digest::*;
