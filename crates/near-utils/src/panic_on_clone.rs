use impl_tools::autoimpl;
use near_sdk::{env, near};

/// This struct is used as a tool to make it possible to derive borsh
/// serialization in such a way where serialization can take over a reference
/// and include it as if it's owned by the struct/enum being serialized.
/// The reference can be taken in the function [`PanicOnClone::from_ref()`].
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

#[cfg(test)]
mod tests {
    use super::*;

    use core::ptr;

    #[test]
    fn from_ref() {
        let value = "example".to_string();
        let poc = PanicOnClone::from_ref(&value);
        assert!(ptr::eq(&**poc, &value));
        assert_eq!(&**poc, &value);
    }

    #[test]
    #[should_panic(expected = "PanicOnClone")]
    #[allow(clippy::redundant_clone)]
    fn panics_on_clone() {
        struct NotClonable;

        let _ = PanicOnClone::new(
            // doesn't implement Clone
            NotClonable,
        )
        .clone();
    }
}
