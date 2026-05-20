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
#[repr(transparent)]
pub struct Deadline(
    #[cfg_attr(all(feature = "serde", feature = "abi"), schemars(with = "String"))] DateTime<Utc>,
);

impl Deadline {
    pub const UNIX_EPOCH: Self = Self::new(DateTime::UNIX_EPOCH);
    pub const MAX: Self = Self::new(DateTime::<Utc>::MAX_UTC);

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

#[cfg(any(test, feature = "borsh"))]
const _: () = {
    use defuse_borsh_utils::adapters::{
        BorshDeserializeAs, BorshSerializeAs, TimestampMicroSeconds, TimestampMilliSeconds,
        TimestampNanoSeconds, TimestampSeconds,
    };

    macro_rules! impl_borsh_serde_as {
    ($($a:ident,)+) => {
        const _: () = {
            $(
                impl<I> BorshSerializeAs<Deadline> for $a<I>
                where
                    $a<I>: BorshSerializeAs<DateTime<Utc>>,
                {
                    fn serialize_as<W>(source: &Deadline, writer: &mut W) -> std::io::Result<()>
                    where
                        W: std::io::Write,
                    {
                        Self::serialize_as(&source.0, writer)
                    }
                }

                impl<I> BorshDeserializeAs<Deadline> for $a<I>
                where
                    $a<I>: BorshDeserializeAs<DateTime<Utc>>,
                {
                    fn deserialize_as<R>(reader: &mut R) -> std::io::Result<Deadline>
                    where
                        R: std::io::Read,
                    {
                        Self::deserialize_as(reader).map(Deadline)
                    }
                }
            )*
        };
    };
}

    impl_borsh_serde_as! {
        TimestampSeconds, TimestampMilliSeconds, TimestampMicroSeconds, TimestampNanoSeconds,
    }
};

#[cfg(test)]
mod tests {
    #[test]
    fn schema_as_usage() {
        use super::*;
        use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
        use chrono::TimeZone;
        use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};

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
            pub deadline: Deadline,
        }

        let val = S {
            deadline: Deadline::new(Utc.timestamp_opt(1_600_000_000, 123_456_789).unwrap()),
        };
        let bytes = borsh::to_vec(&val).unwrap();
        let decoded = S::try_from_slice(&bytes).unwrap();
        assert_eq!(val.deadline, decoded.deadline);
    }
}
