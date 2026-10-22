pub trait CheckedDiv<RHS = Self>: Sized {
    fn checked_div(self, rhs: RHS) -> Option<Self>;

    fn checked_div_ceil(self, rhs: RHS) -> Option<Self>;
}

macro_rules! impl_checked_div {
    ($($t:ty),+) => {$(
        impl CheckedDiv for $t {
            #[inline]
            fn checked_div(self, rhs: Self) -> Option<Self> {
                self.checked_div(rhs)
            }

            #[inline]
            fn checked_div_ceil(self, rhs: Self) -> Option<Self> {
                if rhs == 0 {
                    return None;
                }
                Some(self.div_ceil(rhs))
            }
        }
    )+};
}
impl_checked_div!(u8, u16, u32, u64, u128);
//  #![feature(int_roundings)]
// impl_checked_div!(i8, i16, i32, i64, i128);

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(10u8, 3u8, Some(3u8), Some(4u8))]
    #[case(9u8, 3u8, Some(3u8), Some(3u8))]
    #[case(10u8, 0u8, None, None)]
    #[case(0u8, 5u8, Some(0u8), Some(0u8))]
    #[case(1u8, 1u8, Some(1u8), Some(1u8))]
    #[case(u128::MAX, 1u128, Some(u128::MAX), Some(u128::MAX))]
    #[case(u128::MAX, u128::MAX, Some(1u128), Some(1u128))]
    #[case(u128::MAX, 0u128, None, None)]
    fn checked_div_and_ceil<T>(
        #[case] a: T,
        #[case] b: T,
        #[case] expected_div: Option<T>,
        #[case] expected_ceil: Option<T>,
    ) where
        T: CheckedDiv<T> + PartialEq + Debug + Copy,
    {
        assert_eq!(a.checked_div(b), expected_div);
        assert_eq!(a.checked_div_ceil(b), expected_ceil);
    }
}
