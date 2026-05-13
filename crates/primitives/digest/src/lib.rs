#[cfg(all(feature = "near-contract", feature = "sha2"))]
compile_error!("features `near-contract` and `sha2` are mutually exclusive");

#[cfg(all(feature = "near-contract", feature = "sha3"))]
compile_error!("features `near-contract` and `sha3` are mutually exclusive");

#[cfg(feature = "near-contract")]
pub use defuse_near_utils::digest::Sha256;

#[cfg(feature = "sha2")]
pub use sha2::Sha256;

#[cfg(feature = "near-contract")]
pub use defuse_near_utils::digest::Keccak256;

#[cfg(feature = "sha3")]
pub use sha3::Keccak256;
