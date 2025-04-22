use impl_tools::autoimpl;
use near_sdk::{env, near};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[autoimpl(Deref using self.0)]
#[autoimpl(DerefMut using self.0)]
#[autoimpl(AsRef using self.0)]
#[autoimpl(AsMut using self.0)]
#[near(serializers = [borsh])]
#[repr(transparent)] // needed for `transmute()` below
pub struct PanicOnClone<T: ?Sized>(T);

impl<T> PanicOnClone<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    #[inline]
    pub const fn from_ref(value: &T) -> &Self {
        // this is safe due to `#[repr(transparent)]`
        unsafe { ::core::mem::transmute::<&T, &Self>(value) }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for PanicOnClone<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> Clone for PanicOnClone<T> {
    #[track_caller]
    fn clone(&self) -> Self {
        env::panic_str("PanicOnClone")
    }
}
