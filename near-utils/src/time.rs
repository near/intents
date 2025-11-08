use core::{
    ops::{Add, AddAssign},
    time::Duration,
};

use std::{io, sync::LazyLock};

use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters::{BorshDeserializeAs, BorshSerializeAs, TimestampNanoSeconds};
use near_sdk::{env, near};

/// Cached [`env::block_timestamp()`]
pub static BLOCK_TIMESTAMP: LazyLock<DateTime<Utc>> = LazyLock::new(crate::time::now);

pub fn now() -> DateTime<Utc> {
    DateTime::from_timestamp_nanos(
        env::block_timestamp()
            .try_into()
            .unwrap_or_else(|_| unreachable!()),
    )
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[near(serializers=[json])]
#[repr(transparent)]
pub struct Deadline(
    #[cfg_attr(
        all(feature = "abi", not(target_arch = "wasm32")),
        schemars(with = "String")
    )]
    DateTime<Utc>,
);

impl Deadline {
    pub const MAX: Self = Self(DateTime::<Utc>::MAX_UTC);

    pub const fn new(d: DateTime<Utc>) -> Self {
        Self(d)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn now() -> Self {
        Self(BLOCK_TIMESTAMP.clone())
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub fn now() -> Self {
        Self(Utc::now())
    }

    #[inline]
    pub fn timeout(timeout: Duration) -> Self {
        Self::now() + timeout
    }

    #[inline]
    pub fn has_expired(self) -> bool {
        Self::now() > self
    }

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
