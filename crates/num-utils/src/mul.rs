pub trait CheckedMul<RHS = Self>: Sized {
    fn checked_mul(self, rhs: RHS) -> Option<Self>;

    #[inline]
    fn checked_mul_ceil(self, rhs: RHS) -> Option<Self> {
        self.checked_mul(rhs)
    }
}

macro_rules! impl_checked_mul {
    ($($t:ty),+) => {$(
        impl CheckedMul for $t {
            #[inline]
            fn checked_mul(self, rhs: Self) -> Option<Self> {
                self.checked_mul(rhs)
            }
        }
    )+};
}
impl_checked_mul!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128);

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(2u8, 3u8, Some(6u8))]
    #[case(200u8, 2u8, None)]
    #[case(0u8, 0u8, Some(0u8))]
    #[case(-2i8, 3i8, Some(-6i8))]
    #[case(i8::MIN, -1i8, None)]
    #[case(u128::MAX, 2u128, None)]
    #[case(i128::MIN, -1i128, None)]
    fn checked_mul_and_ceil<T>(#[case] a: T, #[case] b: T, #[case] expected: Option<T>)
    where
        T: CheckedMul<T> + PartialEq + Debug + Copy,
    {
        assert_eq!(a.checked_mul(b), expected);
        // No override exists for any of the base integer types, so the
        // default `checked_mul_ceil` impl must always match `checked_mul`.
        assert_eq!(a.checked_mul_ceil(b), expected);
    }
}
