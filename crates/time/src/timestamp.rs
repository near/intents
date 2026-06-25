use std::time::Duration;

use crate::SignedDuration;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(SignedDuration);

impl Timestamp {
    #[must_use]
    #[inline]
    pub const fn from_secs(secs: i64) -> Self {
        Self(SignedDuration::from_secs(secs))
    }

    #[must_use]
    #[inline]
    pub const fn from_millis(millis: i64) -> Self {
        Self(SignedDuration::from_millis(millis))
    }

    #[must_use]
    #[inline]
    pub const fn from_micros(micros: i64) -> Self {
        Self(SignedDuration::from_micros(micros))
    }

    #[must_use]
    #[inline]
    pub const fn from_nanos(nanos: i64) -> Self {
        Self(SignedDuration::from_nanos(nanos))
    }

    #[must_use]
    #[inline]
    pub const fn from_nanos_i128(nanos: i128) -> Self {
        Self(SignedDuration::from_nanos_i128(nanos))
    }

    #[must_use]
    #[inline]
    pub const fn abs_diff(self, rhs: Self) -> Duration {
        self.0.abs_diff(rhs.0)
    }

    #[must_use]
    #[inline]
    pub const fn since(self, rhs: Self) -> SignedDuration {
        self.0.checked_sub(rhs)
    }

    #[must_use]
    #[inline]
    pub const fn as_secs(&self) -> i64 {
        self.0.as_secs()
    }

    #[must_use]
    #[inline]
    pub const fn as_millis(&self) -> i128 {
        self.0.as_millis()
    }

    #[must_use]
    #[inline]
    pub const fn as_micros(&self) -> i128 {
        self.0.as_micros()
    }

    #[must_use]
    #[inline]
    pub const fn as_nanos(&self) -> i128 {
        self.0.as_nanos()
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::time::SystemTime;

    use crate::Overflow;

    impl Timestamp {
        #[must_use]
        #[inline]
        pub fn now() -> Self {
            cfg_select! {
                near => {
                    Self::from_nanos_i128(::near_sdk::env::block_timestamp().into())
                }
                _ => {
                    std::time::SystemTime::now().try_into().expect("system time is before UNIX_EPOCH")
                }
            }
        }

        pub fn elapsed(&self) -> SignedDuration {
            // Self::now()
        }
    }

    impl TryFrom<SystemTime> for Timestamp {
        type Error = Overflow;

        fn try_from(s: SystemTime) -> Result<Self, Self::Error> {
            let r = s.duration_since(SystemTime::UNIX_EPOCH);
            let is_before = r.is_err();
            let dur = SignedDuration::from_unsigned(r.unwrap_or_else(|err| err.duration()))
                .ok_or(Overflow)?;

            Ok(Self(if is_before {
                dur.checked_neg().ok_or(Overflow)?
            } else {
                dur
            }))
        }
    }
};
