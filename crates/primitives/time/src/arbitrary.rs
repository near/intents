use arbitrary::{Arbitrary, Result, Unstructured};
use arbitrary_with::ArbitraryAs;

use crate::{Overflow, Timestamp};

impl<'a> Arbitrary<'a> for Timestamp {
    #[inline]
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let nanos = u.int_in_range(Self::MIN.as_nanos()..=Self::MAX.as_nanos())?;
        Ok(Self::from_nanos(nanos).ok_or(Overflow).unwrap())
    }
}

pub struct RangeNanos<const FROM: i128, const TO: i128 = { Timestamp::MAX.as_nanos() }>;

impl<'a, const FROM: i128, const TO: i128> ArbitraryAs<'a, Timestamp> for RangeNanos<FROM, TO> {
    #[inline]
    fn arbitrary_as(u: &mut Unstructured<'a>) -> Result<Timestamp> {
        let nanos = u.int_in_range(FROM..=TO)?;
        Ok(Timestamp::from_nanos(nanos).ok_or(Overflow).unwrap())
    }
}
