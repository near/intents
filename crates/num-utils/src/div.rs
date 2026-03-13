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
