pub use chrono;

use chrono::{SubsecRound, TimeDelta, TimeZone, Utc};
use core::{
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration,
};

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(
        feature = "abi",
        derive(::schemars::JsonSchema),
        schemars(example = "Self::example")
    )
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct DateTime(
    #[cfg_attr(all(feature = "serde", feature = "abi"), schemars(with = "String"))]
    ::chrono::DateTime<Utc>,
);

impl DateTime {
    pub const UNIX_EPOCH: Self = Self(::chrono::DateTime::UNIX_EPOCH);
    pub const MAX: Self = Self(::chrono::DateTime::<Utc>::MAX_UTC);

    #[cfg(feature = "now")]
    #[must_use]
    #[inline]
    pub fn now() -> Self {
        Self(cfg_select! {
            near => {
                ::chrono::DateTime::from_timestamp_nanos(
                    ::near_sdk::env::block_timestamp()
                        .try_into()
                        .expect("out of range")
                )
            }
            _ => Utc::now(),
        })
    }

    #[cfg(feature = "now")]
    #[must_use]
    #[inline]
    pub fn timeout(timeout: Duration) -> Self {
        Self::now() + timeout
    }

    #[cfg(feature = "now")]
    #[must_use]
    #[inline]
    pub fn has_passed(&self) -> bool {
        *self < Self::now()
    }

    /// Truncate `Deadline` down to seconds part.
    /// E.g. `2026-03-10T09:32:16.123Z` would be truncated down to
    /// `2026-03-10T09:32:16Z`
    #[must_use]
    #[inline]
    pub fn trunc_subsecs(self) -> Self {
        Self(self.into_inner().trunc_subsecs(0))
    }

    #[must_use]
    #[inline]
    pub const fn into_inner(self) -> ::chrono::DateTime<Utc> {
        self.0
    }

    #[cfg(all(feature = "serde", feature = "abi"))]
    fn example() -> Self {
        Utc.with_ymd_and_hms(2026, 6, 20, 19, 15, 37)
            .unwrap()
            .into()
    }
}

impl<Tz: TimeZone> From<::chrono::DateTime<Tz>> for DateTime {
    fn from(value: ::chrono::DateTime<Tz>) -> Self {
        Self(value.to_utc())
    }
}

impl From<DateTime> for ::chrono::DateTime<Utc> {
    fn from(value: DateTime) -> Self {
        value.into_inner()
    }
}

impl Add<Duration> for DateTime {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Add<TimeDelta> for DateTime {
    type Output = Self;

    fn add(self, rhs: TimeDelta) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<Duration> for DateTime {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Sub<TimeDelta> for DateTime {
    type Output = Self;

    fn sub(self, rhs: TimeDelta) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl AddAssign<Duration> for DateTime {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs;
    }
}

impl AddAssign<TimeDelta> for DateTime {
    #[inline]
    fn add_assign(&mut self, rhs: TimeDelta) {
        self.0 += rhs;
    }
}

impl SubAssign<Duration> for DateTime {
    fn sub_assign(&mut self, rhs: Duration) {
        self.0 -= rhs;
    }
}

impl SubAssign<TimeDelta> for DateTime {
    fn sub_assign(&mut self, rhs: TimeDelta) {
        self.0 -= rhs;
    }
}

#[cfg(feature = "serde")]
const _: () = {
    use serde_with::{
        DeserializeAs, SerializeAs, TimestampMicroSeconds, TimestampMicroSecondsWithFrac,
        TimestampMilliSeconds, TimestampMilliSecondsWithFrac, TimestampNanoSeconds,
        TimestampNanoSecondsWithFrac, TimestampSeconds, TimestampSecondsWithFrac, formats,
    };

    macro_rules! impl_serde_as {
        ($($a:ident,)+) => {$(
            impl<FORMAT: formats::Format, STRICTNESS: formats::Strictness> SerializeAs<DateTime>
                for $a<FORMAT, STRICTNESS>
            where
                $a<FORMAT, STRICTNESS>: SerializeAs<::chrono::DateTime<Utc>>,
            {
                fn serialize_as<S>(source: &DateTime, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    Self::serialize_as(&source.0, serializer)
                }
            }

            impl<'de, FORMAT: formats::Format, STRICTNESS: formats::Strictness> DeserializeAs<'de, DateTime>
                for $a<FORMAT, STRICTNESS>
            where
                $a<FORMAT, STRICTNESS>: DeserializeAs<'de, ::chrono::DateTime<Utc>>,
            {
                fn deserialize_as<D>(deserializer: D) -> Result<DateTime, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    Self::deserialize_as(deserializer).map(DateTime)
                }
            }
        )+};
    }

    impl_serde_as! {
        TimestampSeconds, TimestampSecondsWithFrac,
        TimestampMilliSeconds, TimestampMilliSecondsWithFrac,
        TimestampMicroSeconds, TimestampMicroSecondsWithFrac,
        TimestampNanoSeconds, TimestampNanoSecondsWithFrac,
    }
};

#[cfg(feature = "borsh")]
const _: () = {
    use defuse_borsh_utils::adapters::{
        BorshDeserializeAs, BorshSerializeAs, TimestampMicroSeconds, TimestampMilliSeconds,
        TimestampNanoSeconds, TimestampSeconds,
    };

    macro_rules! impl_borsh_serde_as {
    ($($a:ident,)+) => {$(
        impl<I> BorshSerializeAs<DateTime> for $a<I>
        where
            $a<I>: BorshSerializeAs<::chrono::DateTime<Utc>>,
        {
            fn serialize_as<W>(source: &DateTime, writer: &mut W) -> std::io::Result<()>
            where
                W: std::io::Write,
            {
                Self::serialize_as(&source.0, writer)
            }
        }

        impl<I> BorshDeserializeAs<DateTime> for $a<I>
        where
            $a<I>: BorshDeserializeAs<::chrono::DateTime<Utc>>,
        {
            fn deserialize_as<R>(reader: &mut R) -> std::io::Result<DateTime>
            where
                R: std::io::Read,
            {
                Self::deserialize_as(reader).map(DateTime)
            }
        }
    )*};
}

    impl_borsh_serde_as! {
        TimestampSeconds,
        TimestampMilliSeconds,
        TimestampMicroSeconds,
        TimestampNanoSeconds,
    }
};

#[cfg(test)]
mod tests {
    use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
    use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};

    use super::*;

    #[test]
    fn borsh_schema_as_usage() {
        #[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
        struct S {
            #[borsh(
                serialize_with = "As::<TimestampNanoSeconds>::serialize",
                deserialize_with = "As::<TimestampNanoSeconds>::deserialize",
                schema(with_funcs(
                    declaration = "As::<TimestampNanoSeconds>::declaration",
                    definitions = "As::<TimestampNanoSeconds>::add_definitions_recursively",
                ))
            )]
            pub deadline: DateTime,
        }

        let val = S {
            deadline: Utc
                .timestamp_opt(1_600_000_000, 123_456_789)
                .unwrap()
                .into(),
        };
        let bytes = borsh::to_vec(&val).unwrap();
        let decoded = S::try_from_slice(&bytes).unwrap();
        assert_eq!(val.deadline, decoded.deadline);
    }
}
