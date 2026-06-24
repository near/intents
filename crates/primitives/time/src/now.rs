use core::{ops::Add, time::Duration};

use jiff::Timestamp;

pub trait Now: Sized {
    #[must_use]
    fn now() -> Self;

    #[must_use]
    #[inline]
    fn timeout(timeout: Duration) -> Self
    where
        Self: Add<Duration, Output = Self>,
    {
        Self::now() + timeout
    }

    #[inline]
    fn has_passed(&self) -> bool
    where
        Self: PartialOrd,
    {
        *self < Self::now()
    }
}

impl Now for Timestamp {
    #[track_caller]
    #[inline]
    fn now() -> Self {
        cfg_select! {
            near => {
                Timestamp::from_nanosecond(
                    ::near_sdk::env::block_timestamp().into(),
                ).expect("UNIX timestamp: out of range")
            }
            _ => Timestamp::now(),
        }
    }
}
