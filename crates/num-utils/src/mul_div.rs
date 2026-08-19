use core::ops::Mul;

use bnum::{BInt, BUint, cast::As};

pub trait CheckedMulDiv<RHS = Self>: Sized {
    fn checked_mul_div(self, mul: RHS, div: RHS) -> Option<Self>;
    fn checked_mul_div_ceil(self, mul: RHS, div: RHS) -> Option<Self>;
    fn checked_mul_div_euclid(self, mul: RHS, div: RHS) -> Option<Self>;
}

macro_rules! impl_checked_mul_div {
    ($t:ty as $h:ty) => {
        impl CheckedMulDiv for $t {
            #[inline]
            fn checked_mul_div(self, mul: Self, div: Self) -> Option<Self> {
                self.as_::<$h>()
                    .mul(mul.as_::<$h>())
                    .checked_div(div.as_::<$h>())?
                    .try_into()
                    .ok()
            }

            #[inline]
            fn checked_mul_div_ceil(self, mul: Self, div: Self) -> Option<Self> {
                if div == 0 {
                    return None;
                }
                self.as_::<$h>()
                    .mul(mul.as_::<$h>())
                    .div_ceil(div.as_::<$h>())
                    .try_into()
                    .ok()
            }

            #[inline]
            fn checked_mul_div_euclid(self, mul: Self, div: Self) -> Option<Self> {
                if div == 0 {
                    return None;
                }
                self.as_::<$h>()
                    .mul(mul.as_::<$h>())
                    .div_euclid(div.as_::<$h>())
                    .try_into()
                    .ok()
            }
        }
    };
}
impl_checked_mul_div!(u8 as u16);
impl_checked_mul_div!(u16 as u32);
impl_checked_mul_div!(u32 as u64);
impl_checked_mul_div!(u64 as u128);
impl_checked_mul_div!(u128 as BUint<4>);

// #![feature(int_roundings)]
// const _: () = {
//     impl_checked_mul_div!(i8 as i16);
//     impl_checked_mul_div!(i16 as i32);
//     impl_checked_mul_div!(i32 as i64);
//     impl_checked_mul_div!(i64 as i128);
// };
impl_checked_mul_div!(i128 as BInt<4>);

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use rstest::rstest;

    use super::*;

    #[rstest]
    // exact division
    #[case(10u8, 3u8, 5u8, Some(6u8))]
    // truncates like a regular integer division on a non-exact result
    #[case(10u8, 3u8, 7u8, Some(4u8))]
    // self == 0
    #[case(0u8, 3u8, 7u8, Some(0u8))]
    // division by zero
    #[case(10u8, 3u8, 0u8, None)]
    // the `self * mul` intermediate product would overflow the base type,
    // but not the widened intermediate type used internally
    #[case(200u8, 200u8, 200u8, Some(200u8))]
    // final result doesn't fit back into the base type after narrowing,
    // even though the widened multiplication itself didn't overflow
    #[case(200u8, 200u8, 1u8, None)]
    #[case(u16::MAX, u16::MAX, 1u16, None)]
    #[case(u32::MAX, u32::MAX, 1u32, None)]
    #[case(u64::MAX, u64::MAX, 1u64, None)]
    #[case(u128::MAX, u128::MAX, 1u128, None)]
    #[case(u128::MAX, 1u128, u128::MAX, Some(1u128))]
    fn checked_mul_div_unsigned<T>(
        #[case] a: T,
        #[case] mul: T,
        #[case] div: T,
        #[case] expected: Option<T>,
    ) where
        T: CheckedMulDiv<T> + PartialEq + Debug,
    {
        assert_eq!(a.checked_mul_div(mul, div), expected);
    }

    #[rstest]
    // exact division: no rounding needed
    #[case(10u8, 3u8, 5u8, Some(6u8))]
    // rounds up on a non-exact result
    #[case(10u8, 3u8, 7u8, Some(5u8))]
    // self == 0
    #[case(0u8, 3u8, 7u8, Some(0u8))]
    // division by zero
    #[case(10u8, 3u8, 0u8, None)]
    // narrowing overflow after rounding up
    #[case(200u8, 200u8, 1u8, None)]
    fn checked_mul_div_ceil_unsigned<T>(
        #[case] a: T,
        #[case] mul: T,
        #[case] div: T,
        #[case] expected: Option<T>,
    ) where
        T: CheckedMulDiv<T> + PartialEq + Debug,
    {
        assert_eq!(a.checked_mul_div_ceil(mul, div), expected);
    }

    #[rstest]
    // for non-negative operands, euclidean division matches regular division
    #[case(10u8, 3u8, 7u8, Some(4u8))]
    #[case(10u8, 3u8, 0u8, None)]
    #[case(200u8, 200u8, 1u8, None)]
    fn checked_mul_div_euclid_unsigned<T>(
        #[case] a: T,
        #[case] mul: T,
        #[case] div: T,
        #[case] expected: Option<T>,
    ) where
        T: CheckedMulDiv<T> + PartialEq + Debug,
    {
        assert_eq!(a.checked_mul_div_euclid(mul, div), expected);
    }

    #[rstest]
    // truncates toward zero, unlike `checked_mul_div_euclid` below
    #[case(-7i128, 1i128, 2i128, Some(-3i128))]
    #[case(10i128, 3i128, 0i128, None)]
    // widened multiplication doesn't overflow, but narrowing the result
    // back down to i128 does: `-(i128::MIN)` doesn't fit in i128
    #[case(i128::MIN, -1i128, 1i128, None)]
    fn checked_mul_div_signed(
        #[case] a: i128,
        #[case] mul: i128,
        #[case] div: i128,
        #[case] expected: Option<i128>,
    ) {
        assert_eq!(a.checked_mul_div(mul, div), expected);
    }

    #[rstest]
    // rounds away from zero on the positive side of an exact split
    #[case(10i128, 3i128, 7i128, Some(5i128))]
    #[case(10i128, 3i128, 0i128, None)]
    fn checked_mul_div_ceil_signed(
        #[case] a: i128,
        #[case] mul: i128,
        #[case] div: i128,
        #[case] expected: Option<i128>,
    ) {
        assert_eq!(a.checked_mul_div_ceil(mul, div), expected);
    }

    #[rstest]
    // euclidean division always yields a non-negative remainder, so it
    // rounds toward negative infinity rather than truncating toward zero
    #[case(-7i128, 1i128, 2i128, Some(-4i128))]
    #[case(7i128, 1i128, -2i128, Some(-3i128))]
    #[case(-7i128, 1i128, -2i128, Some(4i128))]
    #[case(10i128, 3i128, 0i128, None)]
    // same narrowing overflow as `checked_mul_div`
    #[case(i128::MIN, -1i128, 1i128, None)]
    fn checked_mul_div_euclid_signed(
        #[case] a: i128,
        #[case] mul: i128,
        #[case] div: i128,
        #[case] expected: Option<i128>,
    ) {
        assert_eq!(a.checked_mul_div_euclid(mul, div), expected);
    }
}
