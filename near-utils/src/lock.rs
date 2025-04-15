use std::{io, marker::PhantomData};

use defuse_borsh_utils::r#as::{AsWrap, BorshDeserializeAs, BorshSerializeAs, Same};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    near,
};

/// A persistent lock, which stores its state (whether it's locked or unlocked)
/// on-chain, so that the inner value can be accessed depending on
/// the current state of the lock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct Lock<T> {
    #[serde(
        default,
        // do not serialize `false`
        skip_serializing_if = "::core::ops::Not::not"
    )]
    // TODO: move to the end of struct?
    locked: bool,
    #[serde(flatten)]
    value: T,
}

impl<T> Lock<T> {
    #[must_use]
    #[inline]
    pub const fn new(value: T, locked: bool) -> Self {
        Self { value, locked }
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

    // TODO: do we need this?
    #[inline]
    pub const fn as_inner_unchecked(&self) -> &T {
        &self.value
    }

    // TODO: do we need this?
    #[inline]
    pub const fn as_inner_unchecked_mut(&mut self) -> &mut T {
        &mut self.value
    }

    // TODO: do we need this?
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
        Some(&self.value)
    }

    #[must_use]
    #[inline]
    pub const fn as_locked_mut(&mut self) -> Option<&mut T> {
        if !self.is_locked() {
            return None;
        }
        Some(&mut self.value)
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
        Some(&mut self.value)
    }

    #[inline]
    pub const fn force_lock(&mut self) -> &mut T {
        self.locked = true;
        &mut self.value
    }

    #[must_use]
    #[inline]
    pub const fn as_unlocked(&self) -> Option<&T> {
        if self.is_locked() {
            return None;
        }
        Some(&self.value)
    }

    #[must_use]
    #[inline]
    pub const fn as_unlocked_mut(&mut self) -> Option<&mut T> {
        if self.is_locked() {
            return None;
        }
        Some(&mut self.value)
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
        Some(&mut self.value)
    }

    #[inline]
    pub const fn force_unlock(&mut self) -> &mut T {
        self.locked = false;
        &mut self.value
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

pub struct MaybeLock<T: ?Sized = Same, const DEFAULT_LOCKED: bool = false>(PhantomData<T>);

impl<T, As, const DEFAULT_LOCKED: bool> BorshDeserializeAs<Lock<T>>
    for MaybeLock<As, DEFAULT_LOCKED>
where
    As: BorshDeserializeAs<T>,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Lock<T>>
    where
        R: io::Read,
    {
        Lock::<As>::deserialize_as(reader)
            .or_else(|_| As::deserialize_as(reader).map(|v| Lock::new(v, DEFAULT_LOCKED)))
    }
}

impl<T, As, const DEFAULT_LOCKED: bool> BorshSerializeAs<Lock<T>> for MaybeLock<As, DEFAULT_LOCKED>
where
    As: BorshSerializeAs<T>,
{
    #[inline]
    fn serialize_as<W>(source: &Lock<T>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        Lock::<As>::serialize_as(source, writer)
    }
}
