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
