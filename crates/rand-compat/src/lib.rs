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

pub trait RandCompat: Sized {
    #[cfg(feature = "rand_core_0_6")]
    fn v0_6(self) -> V0_6<Self> {
        V0_6(self)
    }

    #[cfg(feature = "rand_core_0_9")]
    fn v0_9(self) -> V0_9<Self> {
        V0_9(self)
    }

    #[cfg(feature = "rand_core_0_10")]
    fn v0_10(self) -> V0_10<Self>
    where
        Self: crate::rand_core_0_10::TryRng,
    {
        V0_10(self)
    }
}
impl<R> RandCompat for R {}
