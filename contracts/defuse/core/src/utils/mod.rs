mod lock;

#[cfg(feature = "near-contract")]
mod prefix;

pub use lock::Lock;
#[cfg(feature = "near-contract")]
pub use prefix::NestPrefix;
