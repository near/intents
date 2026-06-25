#[cfg(feature = "borsh")]
pub mod borsh;
#[cfg(feature = "serde")]
pub mod serde;

mod error;
use chrono::{DateTime, SubsecRound, TimeDelta, Utc};

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
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    pub const MIN: Self = Self(DateTime::<Utc>::MIN_UTC);
    pub const UNIX_EPOCH: Self = Self(DateTime::<Utc>::UNIX_EPOCH);
    pub const MAX: Self = Self(DateTime::<Utc>::MAX_UTC);

    #[must_use]
    #[inline]
    pub fn from_nanos(nanos: i128) -> Option<Self> {
        // `DateTime::from_timestamp_nanos` panics on overflows
        let secs: i64 = nanos.div_euclid(1_000_000_000).try_into().ok()?;
        let nsecs = nanos.rem_euclid(1_000_000_000) as u32;
        DateTime::from_timestamp(secs, nsecs).map(Self)
    }

    #[must_use]
    #[inline]
    pub fn from_micros(micros: i64) -> Option<Self> {
        DateTime::from_timestamp_micros(micros).map(Self)
    }

    #[must_use]
    #[inline]
    pub fn from_millis(millis: i64) -> Option<Self> {
        DateTime::from_timestamp_millis(millis).map(Self)
    }

    #[must_use]
    #[inline]
    pub fn from_secs(secs: i64) -> Option<Self> {
        DateTime::from_timestamp_secs(secs).map(Self)
    }

    #[must_use]
    #[inline]
    pub fn checked_add_unsigned(self, rhs: Duration) -> Option<Self> {
        let rhs = TimeDelta::from_std(rhs).ok()?;
        self.0.checked_add_signed(rhs).map(Self)
    }

    #[must_use]
    #[inline]
    pub fn checked_sub_unsigned(self, rhs: Duration) -> Option<Self> {
        let rhs = TimeDelta::from_std(rhs).ok()?;
        self.0.checked_sub_signed(rhs).map(Self)
    }

    #[inline]
    pub fn duration_since(&self, other: Self) -> Result<Duration, Duration> {
        let dur = self.0.signed_duration_since(other.0);
        dur.to_std().map_err(|_| (-dur).to_std().unwrap())
    }

    #[must_use]
    #[inline]
    pub fn truncate_subsecs(self) -> Self {
        Self(self.0.trunc_subsecs(0))
    }

    #[must_use]
    #[inline]
    pub fn as_nanos(&self) -> i128 {
        self.0.timestamp_nanos_opt().unwrap().into()
    }

    #[must_use]
    #[inline]
    pub fn as_micros(&self) -> i64 {
        self.0.timestamp_micros()
    }

    #[must_use]
    #[inline]
    pub fn as_millis(&self) -> i64 {
        self.0.timestamp_millis()
    }

    #[must_use]
    #[inline]
    pub fn as_secs(&self) -> i64 {
        self.0.timestamp()
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
        self.checked_add_unsigned(rhs).ok_or(Overflow).unwrap()
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
        self.checked_sub_unsigned(rhs).ok_or(Overflow).unwrap()
    }
}

impl SubAssign<Duration> for Timestamp {
    #[inline]
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl FromStr for Timestamp {
    type Err = chrono::ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
            cfg_select! {
                near => {
                    Self::from_nanos(
                        ::near_sdk::env::block_timestamp().into(),
                    ).ok_or(Overflow).unwrap()
                }
                _ => Self(Utc::now()),
            }
        }

        #[must_use]
        #[inline]
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
