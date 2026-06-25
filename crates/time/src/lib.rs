mod duration;
mod error;
mod timestamp;
pub use self::{duration::*, timestamp::*, error::*};

#[cfg(feature = "borsh")]
pub mod borsh;
#[cfg(feature = "serde")]
pub mod serde;

use chrono::{DateTime, SubsecRound, Utc};
use core::{
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration,
};
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(SignedDuration);
// #[cfg_attr(all(feature = "serde", feature = "abi"), schemars(with = "String"))] DateTime<Utc>,

impl Timestamp {
    pub const UNIX_EPOCH: Self = Self::new(DateTime::UNIX_EPOCH);
    pub const MAX: Self = Self::new(DateTime::<Utc>::MAX_UTC);

    pub const fn new(d: DateTime<Utc>) -> Self {
        Self(d)
    }

    #[must_use]
    #[inline]
    pub fn now() -> Self {
        Self(cfg_select! {
            near => {
                DateTime::from_timestamp_nanos(
                    ::near_sdk::env::block_timestamp()
                        .try_into()
                        .unwrap_or_else(|_| unreachable!()),
                )
            }
            _ => Utc::now()
        })
    }

    #[must_use]
    #[inline]
    pub fn timeout(timeout: Duration) -> Self {
        Self::now() + timeout
    }

    #[must_use]
    #[inline]
    pub fn has_expired(self) -> bool {
        Self::now() > self
    }

    /// Truncate `Deadline` down to seconds part.
    /// E.g. `2026-03-10T09:32:16.123Z` would be truncated down to
    /// `2026-03-10T09:32:16Z`
    #[must_use]
    #[inline]
    pub fn trunc_subsecs(self) -> Self {
        Self::new(self.into_timestamp().trunc_subsecs(0))
    }

    #[must_use]
    #[inline]
    pub const fn into_timestamp(self) -> DateTime<Utc> {
        self.0
    }
}

impl Add<Duration> for Timestamp {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl AddAssign<Duration> for Timestamp {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs;
    }
}

impl SubAssign<Duration> for Timestamp {
    fn sub_assign(&mut self, rhs: Duration) {
        self.0 -= rhs;
    }
}
