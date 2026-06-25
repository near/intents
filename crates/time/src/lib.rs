#[cfg(feature = "serde")]
pub mod serde;

mod error;
pub use self::error::*;

use core::{
    fmt::{self, Display},
    ops::{Add, AddAssign, Sub, SubAssign},
    str::FromStr,
    time::Duration,
};

#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr)
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub(crate) jiff::Timestamp);

impl Timestamp {
    pub const MIN: Self = Self(jiff::Timestamp::MIN);
    pub const UNIX_EPOCH: Self = Self(jiff::Timestamp::UNIX_EPOCH);
    pub const MAX: Self = Self(jiff::Timestamp::MAX);

    #[must_use]
    #[inline]
    pub fn from_nanos(nanos: i128) -> Option<Self> {
        jiff::Timestamp::from_nanosecond(nanos).ok().map(Self)
    }

    #[must_use]
    #[inline]
    pub fn checked_add(self, rhs: Duration) -> Option<Self> {
        todo!()
    }

    #[must_use]
    #[inline]
    pub fn checked_sub(self, rhs: Duration) -> Option<Self> {
        todo!()
    }

    #[must_use]
    #[inline]
    pub fn as_nanos(&self) -> i128 {
        self.0.as_nanosecond()
    }
}

impl Default for Timestamp {
    #[inline]
    fn default() -> Self {
        Self::UNIX_EPOCH
    }
}

impl Add<Duration> for Timestamp {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Duration) -> Self::Output {
        self.checked_add(rhs).ok_or(Overflow).unwrap()
    }
}

impl AddAssign<Duration> for Timestamp {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Duration) -> Self::Output {
        self.checked_sub(rhs).ok_or(Overflow).unwrap()
    }
}

impl SubAssign<Duration> for Timestamp {
    #[inline]
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl FromStr for Timestamp {
    type Err = jiff::Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO
        // jiff::fmt::temporal::
        s.parse().map(Self)
    }
}

impl Display for Timestamp {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "std")]
const _: () = {
    impl Timestamp {
        #[must_use]
        #[inline]
        pub fn now() -> Self {
            Self(cfg_select! {
                // TODO
                // near => {
                //     Self::
                // }
                _ => jiff::Timestamp::now(),
            })
        }

        pub fn timeout(duration: Duration) -> Self {
            Self::now() + duration
        }

        #[must_use]
        #[inline]
        pub fn has_passed(&self) -> bool {
            *self < Self::now()
        }
    }
};

#[cfg(feature = "arbitrary")]
const _: () = {
    use arbitrary::Arbitrary;

    impl<'a> Arbitrary<'a> for Timestamp {
        #[inline]
        fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
            let nanos = u.int_in_range(Self::MIN.as_nanos()..=Self::MAX.as_nanos())?;
            Ok(Self::from_nanos(nanos).expect("nanos overflow"))
        }
    }
};

// use chrono::{DateTime, SubsecRound, Utc};
// use core::{
//     ops::{Add, AddAssign, Sub, SubAssign},
//     time::Duration,
// };
// #[cfg_attr(
//     feature = "serde",
//     derive(::serde::Serialize, ::serde::Deserialize),
//     cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
// )]
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// #[repr(transparent)]
// pub struct Timestamp(
//     #[cfg_attr(all(feature = "serde", feature = "abi"), schemars(with = "String"))] DateTime<Utc>,
// );

// impl Timestamp {
//     pub const UNIX_EPOCH: Self = Self::new(DateTime::UNIX_EPOCH);
//     // pub const MAX: Self = Self::new(DateTime::<Utc>::MAX_UTC);

//     pub const fn new(d: DateTime<Utc>) -> Self {
//         Self(d)
//     }

//     #[cfg(feature = "std")]
//     #[must_use]
//     #[inline]
//     pub fn now() -> Self {
//         Self(cfg_select! {
//             near => {
//                 DateTime::from_timestamp_nanos(
//                     env::block_timestamp()
//                         .try_into()
//                         .unwrap_or_else(|_| unreachable!()),
//                 )
//             }
//             _ => Utc::now(),
//         })
//     }

//     #[must_use]
//     #[inline]
//     pub fn timeout(timeout: Duration) -> Self {
//         Self::now() + timeout
//     }

//     #[must_use]
//     #[inline]
//     pub fn has_expired(self) -> bool {
//         Self::now() > self
//     }

//     /// Truncate `Deadline` down to seconds part.
//     /// E.g. `2026-03-10T09:32:16.123Z` would be truncated down to
//     /// `2026-03-10T09:32:16Z`
//     #[must_use]
//     #[inline]
//     pub fn trunc_subsecs(self) -> Self {
//         Self::new(self.into_timestamp().trunc_subsecs(0))
//     }

//     #[must_use]
//     #[inline]
//     pub const fn into_timestamp(self) -> DateTime<Utc> {
//         self.0
//     }
// }

// impl Add<Duration> for Timestamp {
//     type Output = Self;

//     #[inline]
//     fn add(self, rhs: Duration) -> Self::Output {
//         Self(self.0 + rhs)
//     }
// }

// impl Sub<Duration> for Timestamp {
//     type Output = Self;

//     fn sub(self, rhs: Duration) -> Self::Output {
//         Self(self.0 - rhs)
//     }
// }

// impl AddAssign<Duration> for Timestamp {
//     #[inline]
//     fn add_assign(&mut self, rhs: Duration) {
//         self.0 += rhs;
//     }
// }

// impl SubAssign<Duration> for Timestamp {
//     fn sub_assign(&mut self, rhs: Duration) {
//         self.0 -= rhs;
//     }
// }

// #[cfg(any(test, feature = "borsh"))]
// const _: () = {
//     use defuse_borsh_utils::adapters::{
//         BorshDeserializeAs, BorshSerializeAs, TimestampMicroSeconds, TimestampMilliSeconds,
//         TimestampNanoSeconds, TimestampSeconds,
//     };

//     macro_rules! impl_borsh_serde_as {
//     ($($a:ident,)+) => {
//         const _: () = {
//             $(
//                 impl<I> BorshSerializeAs<Deadline> for $a<I>
//                 where
//                     $a<I>: BorshSerializeAs<DateTime<Utc>>,
//                 {
//                     fn serialize_as<W>(source: &Deadline, writer: &mut W) -> std::io::Result<()>
//                     where
//                         W: std::io::Write,
//                     {
//                         Self::serialize_as(&source.0, writer)
//                     }
//                 }

//                 impl<I> BorshDeserializeAs<Deadline> for $a<I>
//                 where
//                     $a<I>: BorshDeserializeAs<DateTime<Utc>>,
//                 {
//                     fn deserialize_as<R>(reader: &mut R) -> std::io::Result<Deadline>
//                     where
//                         R: std::io::Read,
//                     {
//                         Self::deserialize_as(reader).map(Deadline)
//                     }
//                 }
//             )*
//         };
//     };
// }

//     impl_borsh_serde_as! {
//         TimestampSeconds, TimestampMilliSeconds, TimestampMicroSeconds, TimestampNanoSeconds,
//     }
// };

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn schema_as_usage() {
//         use super::*;
//         use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
//         use chrono::TimeZone;
//         use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};

//         #[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
//         struct S {
//             #[borsh(
//                 serialize_with = "As::<TimestampNanoSeconds>::serialize",
//                 deserialize_with = "As::<TimestampNanoSeconds>::deserialize",
//                 schema(with_funcs(
//                     declaration = "As::<TimestampNanoSeconds>::declaration",
//                     definitions = "As::<TimestampNanoSeconds>::add_definitions_recursively",
//                 ))
//             )]
//             pub deadline: Timestamp,
//         }

//         let val = S {
//             deadline: Timestamp::new(Utc.timestamp_opt(1_600_000_000, 123_456_789).unwrap()),
//         };
//         let bytes = borsh::to_vec(&val).unwrap();
//         let decoded = S::try_from_slice(&bytes).unwrap();
//         assert_eq!(val.deadline, decoded.deadline);
//     }
// }
