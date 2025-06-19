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
    pub const fn new(value: T, locked: bool) -> Self {
        Self { locked, value }
    }

    #[must_use]
    #[inline]
    pub const fn unlocked(value: T) -> Self {
        Self::new(value, false)
    }

    #[must_use]
    #[inline]
    pub const fn locked(value: T) -> Self {
        Self::new(value, true)
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
    pub const fn as_locked_or_mut(&mut self, force: bool) -> Option<&mut T> {
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
    pub const fn as_unlocked(&self) -> Option<&T> {
        if self.is_locked() {
            return None;
        }
        Some(self.as_inner_unchecked())
    }

    #[must_use]
    #[inline]
    pub const fn as_unlocked_mut(&mut self) -> Option<&mut T> {
        if self.is_locked() {
            return None;
        }
        Some(self.as_inner_unchecked_mut())
    }

    #[must_use]
    #[inline]
    pub const fn as_unlocked_or_mut(&mut self, force: bool) -> Option<&mut T> {
        if force {
            Some(self.as_inner_unchecked_mut())
        } else {
            self.as_unlocked_mut()
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
    let mut a = Lock::new(0, false);

    assert!(!a.is_locked());
    assert_eq!(a.unlock(), None);

    assert_eq!(a.as_unlocked().copied(), Some(0));
    *a.as_unlocked_mut().unwrap() += 1;
    assert_eq!(*a.as_inner_unchecked(), 1);

    assert_eq!(a.lock().copied(), Some(1));
    assert!(a.is_locked());

    assert_eq!(a.as_locked().copied(), Some(1));
    *a.as_locked_mut().unwrap() += 1;
    assert_eq!(*a.as_inner_unchecked(), 2);
}
