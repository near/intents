
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
