use core::{
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration,
};
use std::io;

use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters::{
    BorshDeserializeAs, BorshSerializeAs, TimestampMicroSeconds, TimestampMilliSeconds,
    TimestampNanoSeconds, TimestampSeconds,
};
use near_sdk::near;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[near(serializers = [json])]
#[repr(transparent)]
pub struct Deadline(#[cfg_attr(feature = "abi", schemars(with = "String"))] DateTime<Utc>);

impl Deadline {
    pub const MIN: Self = Self(DateTime::UNIX_EPOCH);
    pub const MAX: Self = Self(DateTime::<Utc>::MAX_UTC);

    pub const fn new(d: DateTime<Utc>) -> Self {
        Self(d)
    }

    #[cfg(target_arch = "wasm32")]
    #[must_use]
    pub fn now() -> Self {
        Self(defuse_near_utils::time::now())
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

impl Sub<Duration> for Deadline {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl AddAssign<Duration> for Deadline {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs;
    }
}

impl SubAssign<Duration> for Deadline {
    fn sub_assign(&mut self, rhs: Duration) {
        self.0 -= rhs;
    }
}

macro_rules! impl_borsh_serde_as {
    ($($a:ident,)+) => {$(
        impl<I> BorshSerializeAs<Deadline> for $a<I>
        where
            $a<I>: BorshSerializeAs<DateTime<Utc>>,
        {
            fn serialize_as<W>(source: &Deadline, writer: &mut W) -> io::Result<()>
            where
                W: io::Write,
            {
                Self::serialize_as(&source.0, writer)
            }
        }

        impl<I> BorshDeserializeAs<Deadline> for $a<I>
        where
            $a<I>: BorshDeserializeAs<DateTime<Utc>>,
        {
            fn deserialize_as<R>(reader: &mut R) -> io::Result<Deadline>
            where
                R: io::Read,
            {
                Self::deserialize_as(reader).map(Deadline)
            }
        }
    )*};
}
impl_borsh_serde_as! {
    TimestampSeconds, TimestampMilliSeconds, TimestampMicroSeconds, TimestampNanoSeconds,
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "abi")]
    #[test]
    fn schema_as_usage() {
        use super::*;
        use chrono::TimeZone;
        use defuse_borsh_utils::adapters::As;
        use near_sdk::borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

        #[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
        #[borsh(crate = "::near_sdk::borsh")]
        struct S {
            #[borsh(
                serialize_with = "As::<TimestampNanoSeconds>::serialize",
                deserialize_with = "As::<TimestampNanoSeconds>::deserialize",
                schema(with_funcs(
                    declaration = "As::<TimestampNanoSeconds>::declaration",
                    definitions = "As::<TimestampNanoSeconds>::add_definitions_recursively",
                ))
            )]
            pub deadline: Deadline,
        }

        let val = S {
            deadline: Deadline::new(Utc.timestamp_opt(1_600_000_000, 123_456_789).unwrap()),
        };
        let bytes = near_sdk::borsh::to_vec(&val).unwrap();
        let decoded = S::try_from_slice(&bytes).unwrap();
        assert_eq!(val.deadline, decoded.deadline);
    }
}
