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
