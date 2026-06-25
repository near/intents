use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};
pub use core::time::Duration;

use crate::Overflow;

const MILLIS_PER_SEC: u32 = 1_000;
const MICROS_PER_SEC: u32 = MILLIS_PER_SEC * 1_000;
const NANOS_PER_SEC: u32 = MICROS_PER_SEC * 1_000;

const NANOS_PER_MICRO: u32 = NANOS_PER_SEC / MICROS_PER_SEC;
const NANOS_PER_MILLI: u32 = NANOS_PER_SEC / MILLIS_PER_SEC;

const SECS_PER_MINUTE: i64 = 60;
const SECS_PER_HOUR: i64 = SECS_PER_MINUTE * 60;

// TODO: PartialEq?
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignedDuration {
    secs: i64,
    nanos: i32,
}

impl SignedDuration {
    pub const MIN: Self = Self::new(i64::MIN, u32::MAX);
    pub const ZERO: Self = Self::from_nanos(0);
    pub const MAX: Self = Self::new(i64::MAX, NANOS_PER_SEC - 1);

    pub const SECOND: Self = Self::from_secs(1);
    pub const MILLISECOND: Self = Self::from_millis(1);
    pub const MICROSECOND: Self = Self::from_micros(1);
    pub const NANOSECOND: Self = Self::from_nanos(1);

    pub const MINUTE: Self = Self::from_mins(1);
    pub const HOUR: Self = Self::from_hours(1);

    #[must_use]
    #[inline]
    pub const fn new(mut secs: i64, mut nanos: u32) -> Self {
        if nanos >= NANOS_PER_SEC {
            secs = secs
                .checked_add((nanos / NANOS_PER_SEC) as i64)
                .expect("overflow in `SignedDuration::new`");
            nanos = nanos % NANOS_PER_SEC;
        }
        Self { secs, nanos }
    }

    #[must_use]
    #[inline]
    pub const fn from_secs(secs: i64) -> Self {
        Self::new(secs, 0)
    }

    /// Creates new `SignedDuration` from specified number of milliseconds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use defuse_time::SignedDuration;
    ///
    /// let duration =
    /// ```
    #[must_use]
    #[inline]
    pub const fn from_millis(millis: i64) -> Self {
        let secs = millis / MILLIS_PER_SEC as i64;
        let subsec_millis = (millis.unsigned_abs() % MILLIS_PER_SEC as u64) as u32;
        let subsec_nanos = subsec_millis * NANOS_PER_MILLI;

        Self::new(secs, subsec_nanos)
    }

    #[must_use]
    #[inline]
    pub const fn from_micros(micros: i64) -> Self {
        let secs = micros / MICROS_PER_SEC as i64;
        let subsec_micros = (micros.unsigned_abs() % MICROS_PER_SEC as u64) as u32;
        let subsec_nanos = subsec_micros * NANOS_PER_MICRO;

        Self::new(secs, subsec_nanos)
    }

    #[must_use]
    #[inline]
    pub const fn from_nanos(nanos: i64) -> Self {
        let secs = nanos / NANOS_PER_SEC as i64;
        let subsec_nanos = (nanos.unsigned_abs() % NANOS_PER_SEC as u64) as u32;

        Self::new(secs, subsec_nanos)
    }

    #[must_use]
    #[inline]
    pub const fn from_nanos_i128(nanos: i128) -> Self {
        let secs = nanos / NANOS_PER_SEC as i128;
        if secs < (i64::MIN as i128) || (i64::MAX as i128) < secs {
            panic!("overflow in `SignedDuration::from_nanos_i128`");
        }
        let subsec_nanos = (nanos.unsigned_abs() % NANOS_PER_SEC as u128) as u32;

        Self::new(secs as i64, subsec_nanos)
    }

    #[must_use]
    #[inline]
    pub const fn from_mins(mins: i64) -> Self {
        if mins > i64::MAX / SECS_PER_MINUTE {
            panic!("overflow in `Duration::from_mins");
        }
        Self::from_secs(mins * SECS_PER_MINUTE)
    }

    #[must_use]
    #[inline]
    pub const fn from_hours(hours: i64) -> Self {
        if hours > i64::MAX / (SECS_PER_HOUR) {
            panic!("overflow in `Duration::from_mins");
        }
        Self::from_secs(hours * SECS_PER_HOUR)
    }

    #[must_use]
    #[inline]
    pub const fn from_unsigned(duration: Duration) -> Option<Self> {
        let secs = duration.as_secs();
        if secs > i64::MAX as u64 {
            return None;
        }
        let nanos = duration.subsec_nanos();
        Some(Self::new(secs as i64, nanos))
    }

    #[must_use]
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.secs == 0 && self.nanos == 0
    }

    #[must_use]
    #[inline]
    pub const fn is_positive(&self) -> bool {
        self.secs.is_positive()
    }

    #[must_use]
    #[inline]
    pub const fn is_negative(&self) -> bool {
        self.secs.is_negative()
    }

    #[must_use]
    #[inline]
    pub const fn as_mins(&self) -> i64 {
        self.secs / SECS_PER_MINUTE
    }

    #[must_use]
    #[inline]
    pub const fn as_secs(&self) -> i64 {
        self.secs
    }

    #[must_use]
    #[inline]
    pub const fn as_millis(&self) -> i128 {
        self.secs as i128 * MILLIS_PER_SEC as i128 + (self.nanos / NANOS_PER_MILLI) as i128
    }

    #[must_use]
    #[inline]
    pub const fn as_micros(&self) -> i128 {
        self.secs as i128 * MICROS_PER_SEC as i128 + (self.nanos / NANOS_PER_MICRO) as i128
    }

    #[must_use]
    #[inline]
    pub const fn as_nanos(&self) -> i128 {
        self.secs as i128 * NANOS_PER_SEC as i128 + self.nanos as i128
    }

    #[must_use]
    #[inline]
    pub const fn checked_neg(&self) -> Option<Self> {
        let Some(secs) = self.secs.checked_neg() else {
            return None;
        };
        Some(Self::new(secs, self.nanos))
    }

    #[must_use]
    #[inline]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        let Some(mut secs) = self.secs.checked_add(rhs.secs) else {
            return None;
        };

        // 1.4 + (-1.5) = -0.1

        let mut nanos = self.nanos;
        if rhs.secs > 0 {
            nanos += rhs.nanos;
            if nanos >= NANOS_PER_SEC {
                nanos -= NANOS_PER_SEC;
                let Some(new_secs) = secs.checked_add(1) else {
                    return None;
                };
                secs = new_secs;
            }
        } else if rhs.secs < 0 {
            nanos.abs_diff(other)
            if nanos >= rhs.nanos {
                nanos -= rhs.nanos;
            } else {
                nanos = rhs.nanos - nanos;
            }
            nanos.borrowing_sub(rhs)
            nanos.wrapping_sub(rhs.nanos) as i32;
        }

        let mut nanos: u32;
        if rhs.secs.is_positive() {
            nanos = self.nanos + rhs.nanos;
            if nanos >= NANOS_PER_SEC {
                nanos -= NANOS_PER_SEC;
                let Some(new_secs) = secs.checked_add(1) else {
                    return None;
                };
                secs = new_secs;
            }
        }

        let mut nanos = self.nanos + rhs.nanos;
        if nanos >= NANOS_PER_SEC {
            nanos -= NANOS_PER_SEC;
            let Some(new_secs) = secs.checked_add(1) else {
                return None;
            };
            secs = new_secs;
        }
        debug_assert!(nanos < NANOS_PER_SEC);

        Some(Self::new(secs, nanos))
    }

    #[must_use]
    #[inline]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        let Some(res) = self.checked_add(rhs) else {
            return Self::MAX;
        };
        res
    }

    #[must_use]
    #[inline]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        let Some(rhs) = rhs.checked_neg() else {
            return None;
        };
        self.checked_add(rhs)
    }

    #[must_use]
    #[inline]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        let Some(res) = self.checked_sub(rhs) else {
            return Self::MIN;
        };
        res
    }

    #[must_use]
    #[inline]
    pub const fn abs(self) -> Self {
        Self::new(self.secs.abs(), self.nanos)
    }

    #[must_use]
    #[inline]
    pub const fn abs_diff(self, rhs: Self) -> Duration {
        if let Some(res) = self.checked_sub(rhs) {
            res
        } else {
            rhs.checked_sub(self).unwrap()
        }
        .unsigned_abs()
    }

    #[must_use]
    #[inline]
    pub const fn unsigned_abs(self) -> Duration {
        Duration::new(self.secs.unsigned_abs(), self.nanos)
    }

    #[must_use]
    #[inline]
    pub const fn to_unsigned(&self) -> Option<Duration> {
        if self.is_negative() {
            return None;
        }
        Some(self.unsigned_abs())
    }
}

impl Neg for SignedDuration {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        self.checked_neg().ok_or(Overflow).unwrap()
    }
}

impl Add for SignedDuration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).ok_or(Overflow).unwrap()
    }
}

impl AddAssign for SignedDuration {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

impl Sub for SignedDuration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).ok_or(Overflow).unwrap()
    }
}

impl SubAssign for SignedDuration {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs
    }
}

impl TryFrom<Duration> for SignedDuration {
    type Error = Overflow;

    #[inline]
    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        Self::from_unsigned(value).ok_or(Overflow)
    }
}

impl TryFrom<SignedDuration> for Duration {
    type Error = Overflow;

    #[inline]
    fn try_from(value: SignedDuration) -> Result<Self, Self::Error> {
        value.to_unsigned().ok_or(Overflow)
    }
}
