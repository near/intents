#[cfg(feature = "arbitrary")]
pub mod arbitrary;
#[cfg(feature = "borsh")]
pub mod borsh;
#[cfg(feature = "serde")]
pub mod serde;

mod error;
pub use self::error::*;

use core::{
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration,
};

#[cfg_attr(
    feature = "serde",
    derive(::serde_with::SerializeDisplay, ::serde_with::DeserializeFromStr)
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(::time::Timestamp);

impl Timestamp {
    pub const MIN: Self = Self(::time::Timestamp::MIN);
    pub const UNIX_EPOCH: Self = Self(::time::Timestamp::UNIX_EPOCH);
    pub const MAX: Self = Self(::time::Timestamp::MAX);

    #[must_use]
    #[inline]
    pub const fn from_nanos(nanos: i128) -> Option<Self> {
        let Ok(ts) = ::time::Timestamp::from_nanoseconds(nanos) else {
            return None;
        };
        Some(Self(ts))
    }

    #[must_use]
    #[inline]
    pub const fn from_micros(micros: i128) -> Option<Self> {
        let Ok(ts) = ::time::Timestamp::from_microseconds(micros) else {
            return None;
        };
        Some(Self(ts))
    }

    #[must_use]
    #[inline]
    pub const fn from_millis(millis: i64) -> Option<Self> {
        let Ok(ts) = ::time::Timestamp::from_milliseconds(millis) else {
            return None;
        };
        Some(Self(ts))
    }

    #[must_use]
    #[inline]
    pub const fn from_secs(secs: i64) -> Option<Self> {
        let Ok(ts) = ::time::Timestamp::from_seconds(secs) else {
            return None;
        };
        Some(Self(ts))
    }
    #[must_use]
    #[inline]
    pub fn checked_add_unsigned(self, rhs: Duration) -> Option<Self> {
        let rhs: ::time::Duration = rhs.try_into().ok()?;
        self.0.checked_add(rhs).map(Self)
    }

    #[must_use]
    #[inline]
    pub fn checked_sub_unsigned(self, rhs: Duration) -> Option<Self> {
        let rhs: ::time::Duration = rhs.try_into().ok()?;
        self.0.checked_sub(rhs).map(Self)
    }

    #[inline]
    pub fn duration_since(&self, other: Self) -> Result<Duration, Duration> {
        let dur = self.0 - other.0;
        if dur.is_negative() {
            return Err(dur.unsigned_abs());
        }
        Ok(dur.unsigned_abs())
    }

    #[must_use]
    #[inline]
    pub const fn truncate_subsecs(self) -> Self {
        let Ok(ts) = self.0.replace_nanosecond(0) else {
            unreachable!()
        };
        Self(ts)
    }

    #[must_use]
    #[inline]
    pub const fn as_nanos(&self) -> i128 {
        self.0.as_nanoseconds()
    }

    #[must_use]
    #[inline]
    pub const fn as_micros(&self) -> i128 {
        self.0.as_microseconds()
    }

    #[must_use]
    #[inline]
    pub const fn as_millis(&self) -> i64 {
        self.0.as_milliseconds()
    }

    #[must_use]
    #[inline]
    pub const fn as_secs(&self) -> i64 {
        self.0.as_seconds()
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

#[cfg(feature = "formatting")]
const _: () = {
    use core::fmt::{self, Display};

    impl Display for Timestamp {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use time::format_description::well_known::Rfc3339;

            f.write_str(&self.0.format(&Rfc3339).map_err(|_| fmt::Error)?)
        }
    }
};

#[cfg(feature = "parsing")]
const _: () = {
    use core::str::FromStr;

    use time::format_description::well_known::Rfc3339;

    impl FromStr for Timestamp {
        type Err = time::error::Parse;

        #[inline]
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            ::time::Timestamp::parse(s, &Rfc3339).map(Self)
        }
    }
};

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
                _ => Self(::time::Timestamp::now()),
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(0, "1970-01-01T00:00:00Z")]
    #[allow(clippy::inconsistent_digit_grouping)]
    #[case(1782395622_123456789, "2026-06-25T13:53:42.123456789Z")]
    fn rfc3339_roundtrip(#[case] nanos: i128, #[case] s: &str) {
        let ts = Timestamp::from_nanos(nanos).unwrap();
        assert_eq!(ts.to_string(), s);

        let got: Timestamp = s.parse().expect("parse");
        assert_eq!(got.as_nanos(), nanos);
    }
}
