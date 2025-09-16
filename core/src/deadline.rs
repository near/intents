use core::{
    ops::{Add, AddAssign},
    time::Duration,
};
use std::io;

use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters::{BorshDeserializeAs, BorshSerializeAs, TimestampNanoSeconds};
use near_sdk::near;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[near(serializers=[json])]
pub struct Deadline(DateTime<Utc>);

impl Deadline {
    pub const MAX: Self = Self(DateTime::<Utc>::MAX_UTC);

    pub const fn new(d: DateTime<Utc>) -> Self {
        Self(d)
    }

    #[cfg(target_arch = "wasm32")]
    #[must_use]
    pub fn now() -> Self {
        Self(defuse_near_utils::BLOCK_TIMESTAMP.clone())
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    #[inline]
    pub fn now() -> Self {
        Self(Utc::now())
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

    #[must_use]
    #[inline]
    pub const fn into_timestamp(self) -> DateTime<Utc> {
        self.0
    }
}

impl Add<Duration> for Deadline {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<Duration> for Deadline {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs;
    }
}

impl BorshSerializeAs<Deadline> for TimestampNanoSeconds {
    fn serialize_as<W>(source: &Deadline, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        Self::serialize_as(&source.0, writer)
    }
}

impl BorshDeserializeAs<Deadline> for TimestampNanoSeconds {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Deadline>
    where
        R: io::Read,
    {
        Self::deserialize_as(reader).map(Deadline)
    }
}
