#![no_std]

#[cfg(feature = "rand_core_0_6")]
mod v0_6;
#[cfg(feature = "rand_core_0_6")]
pub use self::v0_6::*;

#[cfg(feature = "rand_core_0_9")]
mod v0_9;
#[cfg(feature = "rand_core_0_9")]
pub use self::v0_9::*;

#[cfg(feature = "rand_core_0_10")]
mod v0_10;
#[cfg(feature = "rand_core_0_10")]
pub use self::v0_10::*;
