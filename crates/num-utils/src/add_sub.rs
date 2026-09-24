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

macro_rules! impl_checked {
    ($unsigned:ty, $signed:ty) => {
        impl_checked_add!($unsigned, $signed);
        impl_checked_sub!($unsigned, $signed);
    };
}
impl_checked!(u8, i8);
impl_checked!(u16, i16);
impl_checked!(u32, i32);
impl_checked!(u64, i64);
impl_checked!(u128, i128);

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(2u8, 3u8, Some(5u8))]
    #[case(u8::MAX, 1u8, None)]
    #[case(0u8, 0u8, Some(0u8))]
    #[case(2i8, 3i8, Some(5i8))]
    #[case(i8::MAX, 1i8, None)]
    #[case(i8::MIN, -1i8, None)]
    #[case(u128::MAX, 1u128, None)]
    #[case(i128::MIN, -1i128, None)]
    #[case(i128::MAX, -1i128, Some(i128::MAX - 1))]
    fn checked_add_same_type<T>(#[case] a: T, #[case] b: T, #[case] expected: Option<T>)
    where
        T: CheckedAdd<T> + PartialEq + Debug,
    {
        assert_eq!(a.checked_add(b), expected);
    }

    #[rstest]
    #[case(5u8, -3i8, Some(2u8))]
    #[case(0u8, -1i8, None)]
    #[case(250u8, 10i8, None)]
    #[case(250u8, 5i8, Some(255u8))]
    #[case(0u8, i8::MIN, None)]
    fn checked_add_unsigned_plus_signed(
        #[case] a: u8,
        #[case] b: i8,
        #[case] expected: Option<u8>,
    ) {
        // `a.checked_add(b)` would resolve to the inherent `u8::checked_add`
        // (same-type only) rather than this crate's `CheckedAdd<i8>` impl.
        assert_eq!(CheckedAdd::checked_add(a, b), expected);
    }

    #[rstest]
    #[case(-5i8, 3u8, Some(-2i8))]
    #[case(100i8, 30u8, None)]
    #[case(i8::MIN, 1u8, Some(i8::MIN + 1))]
    #[case(-1i8, u8::MAX, None)]
    fn checked_add_signed_plus_unsigned(
        #[case] a: i8,
        #[case] b: u8,
        #[case] expected: Option<i8>,
    ) {
        assert_eq!(CheckedAdd::checked_add(a, b), expected);
    }

    #[rstest]
    #[case(5u8, 3u8, Some(2u8))]
    #[case(3u8, 5u8, None)]
    #[case(0u8, 0u8, Some(0u8))]
    #[case(5i8, 3i8, Some(2i8))]
    #[case(i8::MIN, -1i8, Some(i8::MIN + 1))]
    #[case(i8::MIN, 1i8, None)]
    #[case(0u128, 1u128, None)]
    #[case(i128::MIN, 1i128, None)]
    fn checked_sub_same_type<T>(#[case] a: T, #[case] b: T, #[case] expected: Option<T>)
    where
        T: CheckedSub<T> + PartialEq + Debug,
    {
        assert_eq!(a.checked_sub(b), expected);
    }

    #[rstest]
    #[case(5i8, 3u8, Some(2i8))]
    #[case(i8::MIN, 1u8, None)]
    #[case(-1i8, 1u8, Some(-2i8))]
    // 127 - 255 = -128, landing exactly on `i8::MIN` without underflowing
    #[case(i8::MAX, u8::MAX, Some(i8::MIN))]
    #[case(0i8, u8::MAX, None)]
    fn checked_sub_signed_minus_unsigned(
        #[case] a: i8,
        #[case] b: u8,
        #[case] expected: Option<i8>,
    ) {
        assert_eq!(CheckedSub::checked_sub(a, b), expected);
    }
}
