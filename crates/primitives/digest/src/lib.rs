#[cfg(feature = "near-contract")]
pub use defuse_near_utils::digest::Sha256;

#[cfg(all(feature = "sha2", not(feature = "near-contract")))]
pub use sha2::Sha256;

#[cfg(feature = "near-contract")]
pub use defuse_near_utils::digest::Keccak256;

#[cfg(all(feature = "sha3", not(feature = "near-contract")))]
pub use sha3::Keccak256;

pub use digest::Digest;
