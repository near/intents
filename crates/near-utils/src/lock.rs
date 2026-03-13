use std::io;

use defuse_borsh_utils::adapters::{AsWrap, BorshDeserializeAs, BorshSerializeAs};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    near,
};

/// A persistent lock, which stores its state (whether it's locked or unlocked)
/// on-chain, so that the inner value can be accessed depending on
/// the current state of the lock.
#[derive(Debug, Default, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct Lock<T> {
    #[serde(
        default,
        // do not serialize `false`
        skip_serializing_if = "::core::ops::Not::not"
    )]
    locked: bool,
    #[serde(flatten)]
    value: T,
}

impl<T> Lock<T> {
    #[must_use]
    #[inline]
    pub const fn new(locked: bool, value: T) -> Self {
        Self { locked, value }
    }

    #[must_use]
    #[inline]
    pub const fn unlocked(value: T) -> Self {
        Self::new(false, value)
    }

    #[must_use]
    #[inline]
    pub const fn locked(value: T) -> Self {
        Self::new(true, value)
    }

    #[inline]
    pub const fn set_locked(&mut self, locked: bool) -> &mut Self {
        self.locked = locked;
        self
    }

    /// # Safety
    /// This method bypasses lock state checks. Use only when you need to access
    /// the inner value regardless of lock state, such as for read operations
    /// or when implementing higher-level locking logic.
    #[inline]
    pub const fn as_inner_unchecked(&self) -> &T {
        &self.value
    }

    /// # Safety
    /// This method bypasses lock state checks. Use only when you need mutable access
    /// to the inner value regardless of lock state. Misuse can compromise locking semantics.
    #[inline]
    pub const fn as_inner_unchecked_mut(&mut self) -> &mut T {
        &mut self.value
    }

    #[inline]
    pub fn into_inner_unchecked(self) -> T {
        self.value
    }

    #[must_use]
    #[inline]
    pub const fn is_locked(&self) -> bool {
        self.locked
    }

    #[must_use]
    #[inline]
    pub const fn as_locked(&self) -> Option<&T> {
        if !self.is_locked() {
            return None;
        }
        Some(self.as_inner_unchecked())
    }

    #[must_use]
    #[inline]
    pub const fn as_locked_mut(&mut self) -> Option<&mut T> {
        if !self.is_locked() {
            return None;
        }
        Some(self.as_inner_unchecked_mut())
    }

    #[must_use]
    #[inline]
    pub const fn as_locked_mut_maybe_forced(&mut self, force: bool) -> Option<&mut T> {
        if force {
            Some(self.as_inner_unchecked_mut())
        } else {
            self.as_locked_mut()
        }
    }

    #[must_use]
    #[inline]
    pub fn into_locked(self) -> Option<T> {
        if !self.is_locked() {
            return None;
        }
        Some(self.value)
    }

    #[must_use]
    #[inline]
    pub const fn lock(&mut self) -> Option<&mut T> {
        if self.is_locked() {
            return None;
        }
        self.locked = true;
        Some(self.as_inner_unchecked_mut())
    }

    #[inline]
    pub const fn force_lock(&mut self) -> &mut T {
        self.locked = true;
        self.as_inner_unchecked_mut()
    }

    #[must_use]
    #[inline]
    pub const fn get(&self) -> Option<&T> {
        if self.is_locked() {
            return None;
        }
        Some(self.as_inner_unchecked())
    }

    #[must_use]
    #[inline]
    pub const fn get_mut(&mut self) -> Option<&mut T> {
        if self.is_locked() {
            return None;
        }
        Some(self.as_inner_unchecked_mut())
    }

    #[must_use]
    #[inline]
    pub const fn get_mut_maybe_forced(&mut self, force: bool) -> Option<&mut T> {
        if force {
            Some(self.as_inner_unchecked_mut())
        } else {
            self.get_mut()
        }
    }

    #[must_use]
    #[inline]
    pub fn into_unlocked(self) -> Option<T> {
        if self.is_locked() {
            return None;
        }
        Some(self.value)
    }

    #[must_use]
    #[inline]
    pub const fn unlock(&mut self) -> Option<&mut T> {
        if !self.is_locked() {
            return None;
        }
        self.locked = false;
        Some(self.as_inner_unchecked_mut())
    }

    #[inline]
    pub const fn force_unlock(&mut self) -> &mut T {
        self.locked = false;
        self.as_inner_unchecked_mut()
    }

    #[inline]
    pub const fn as_ref(&self) -> Lock<&T> {
        Lock::new(self.is_locked(), self.as_inner_unchecked())
    }

    #[inline]
    pub const fn as_mut(&mut self) -> Lock<&mut T> {
        Lock::new(self.is_locked(), self.as_inner_unchecked_mut())
    }

    #[inline]
    pub fn map_inner_unchecked<U, F>(self, f: F) -> Lock<U>
    where
        F: FnOnce(T) -> U,
    {
        Lock::new(self.is_locked(), f(self.into_inner_unchecked()))
    }
}

impl<T> From<T> for Lock<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::unlocked(value)
    }
}

impl<T, As> BorshSerializeAs<Lock<T>> for Lock<As>
where
    As: BorshSerializeAs<T>,
{
    #[inline]
    fn serialize_as<W>(source: &Lock<T>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        Lock {
            locked: source.locked,
            value: AsWrap::<&T, &As>::new(&source.value),
        }
        .serialize(writer)
    }
}

impl<T, As> BorshDeserializeAs<Lock<T>> for Lock<As>
where
    As: BorshDeserializeAs<T>,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Lock<T>>
    where
        R: io::Read,
    {
        Lock::<AsWrap<T, As>>::deserialize_reader(reader).map(|v| Lock {
            locked: v.locked,
            value: v.value.into_inner(),
        })
    }
}

#[cfg(test)]
#[test]
fn test() {
    let mut a = Lock::new(false, 0);

    assert!(!a.is_locked());
    assert_eq!(a.unlock(), None);

    assert_eq!(a.get().copied(), Some(0));
    *a.get_mut().unwrap() += 1;
    assert_eq!(*a.as_inner_unchecked(), 1);

    assert_eq!(a.lock().copied(), Some(1));
    assert!(a.is_locked());

    assert_eq!(a.as_locked().copied(), Some(1));
    *a.as_locked_mut().unwrap() += 1;
    assert_eq!(*a.as_inner_unchecked(), 2);
}
