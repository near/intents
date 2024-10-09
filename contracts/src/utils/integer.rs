use core::ops::Mul;

use bnum::{cast::As, BUint, BUintD8};

pub type U256 = BUintD8<32>;

pub trait CheckedAdd<RHS = Self>: Sized {
    fn checked_add(self, rhs: RHS) -> Option<Self>;
}

pub trait CheckedSub<RHS = Self>: Sized {
    fn checked_sub(self, rhs: RHS) -> Option<Self>;
}

macro_rules! impl_checked_add {
    ($unsigned:ty, $signed:ty) => {
        impl CheckedAdd for $unsigned {
            #[inline]
            fn checked_add(self, rhs: Self) -> Option<Self> {
                self.checked_add(rhs)
            }
        }

        impl CheckedAdd<$signed> for $unsigned {
            #[inline]
            fn checked_add(self, rhs: $signed) -> Option<Self> {
                self.checked_add_signed(rhs)
            }
        }

        impl CheckedAdd for $signed {
            #[inline]
            fn checked_add(self, rhs: Self) -> Option<Self> {
                self.checked_add(rhs)
            }
        }

        impl CheckedAdd<$unsigned> for $signed {
            #[inline]
            fn checked_add(self, rhs: $unsigned) -> Option<Self> {
                self.checked_add_unsigned(rhs)
            }
        }
    };
}

macro_rules! impl_checked_sub {
    ($unsigned:ty, $signed:ty) => {
        impl CheckedSub for $unsigned {
            #[inline]
            fn checked_sub(self, rhs: Self) -> Option<Self> {
                self.checked_sub(rhs)
            }
        }

        impl CheckedSub for $signed {
            #[inline]
            fn checked_sub(self, rhs: Self) -> Option<Self> {
                self.checked_sub(rhs)
            }
        }

        impl CheckedSub<$unsigned> for $signed {
            #[inline]
            fn checked_sub(self, rhs: $unsigned) -> Option<Self> {
                self.checked_sub_unsigned(rhs)
            }
        }
    };
}
macro_rules! impl_checked_add_sub {
    ($unsigned:ty, $signed:ty) => {
        impl_checked_add!($unsigned, $signed);
        impl_checked_sub!($unsigned, $signed);
    };
}
impl_checked_add_sub!(u8, i8);
impl_checked_add_sub!(u16, i16);
impl_checked_add_sub!(u32, i32);
impl_checked_add_sub!(u64, i64);
impl_checked_add_sub!(u128, i128);

pub trait CheckedMulDiv<RHS = Self>: Sized {
    fn checked_mul_div(self, mul: RHS, div: RHS) -> Option<Self>;
    fn checked_mul_div_ceil(self, mul: RHS, div: RHS) -> Option<Self>;
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
        }
    };
}
impl_checked_mul_div!(u8 as u16);
impl_checked_mul_div!(u16 as u32);
impl_checked_mul_div!(u32 as u64);
impl_checked_mul_div!(u64 as u128);
impl_checked_mul_div!(u128 as BUint<4>);
