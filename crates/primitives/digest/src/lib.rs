#[cfg(all(feature = "near-contract", feature = "sha2"))]
compile_error!("features `near-contract` and `sha2` are mutually exclusive");

#[cfg(not(any(feature = "near-contract", feature = "sha2")))]
compile_error!("exactly one of `near-contract` or `sha2` must be enabled");

#[cfg(feature = "near-contract")]
pub use defuse_near_utils::digest::Sha256;

#[cfg(feature = "sha2")]
pub use sha2::Sha256;
