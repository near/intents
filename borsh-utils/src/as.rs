//! Analog of [serde_with](https://docs.rs/serde_with) for [borsh](https://docs.rs/borsh)

use std::{io, marker::PhantomData};

use impl_tools::autoimpl;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

pub trait BorshSerializeAs<T: ?Sized> {
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write;
}

pub struct BorshSerializeAsWrap<'a, T: ?Sized, As: ?Sized> {
    value: &'a T,
    marker: PhantomData<As>,
}

impl<'a, T: ?Sized, As: ?Sized> BorshSerializeAsWrap<'a, T, As> {
    #[inline]
    pub const fn new(value: &'a T) -> Self {
        Self {
            value,
            marker: PhantomData,
        }
    }
}

impl<T: ?Sized, As: ?Sized> BorshSerialize for BorshSerializeAsWrap<'_, T, As>
where
    As: BorshSerializeAs<T>,
{
    #[inline]
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        As::serialize_as(self.value, writer)
    }
}

pub trait BorshDeserializeAs<T> {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<T>
    where
        R: io::Read;
}

// TODO
#[autoimpl(Deref using self.value)]
#[autoimpl(DerefMut using self.value)]
#[derive(Debug, Clone, Copy)]
pub struct BorshDeserializeAsWrap<T, As: ?Sized> {
    value: T,
    marker: PhantomData<As>,
}

impl<T, As: ?Sized> BorshDeserializeAsWrap<T, As> {
    #[must_use]
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            value,
            marker: PhantomData,
        }
    }

    /// Return the inner value of type `T`.
    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T, As: ?Sized> From<T> for BorshDeserializeAsWrap<T, As> {
    #[inline]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T, As> BorshDeserialize for BorshDeserializeAsWrap<T, As>
where
    As: BorshDeserializeAs<T> + ?Sized,
{
    #[inline]
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        As::deserialize_as(reader).map(|value| Self {
            value,
            marker: PhantomData,
        })
    }
}

impl<T, As> BorshSerialize for BorshDeserializeAsWrap<T, As>
where
    As: BorshSerializeAs<T> + ?Sized,
{
    #[inline]
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        As::serialize_as(&self.value, writer)
    }
}

pub struct As<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> As<T> {
    #[inline]
    pub fn serialize<U, W>(obj: &U, writer: &mut W) -> io::Result<()>
    where
        T: BorshSerializeAs<U>,
        W: io::Write,
        U: ?Sized,
    {
        T::serialize_as(obj, writer)
    }

    #[inline]
    pub fn deserialize<R, U>(reader: &mut R) -> io::Result<U>
    where
        T: BorshDeserializeAs<U>,
        R: io::Read,
    {
        T::deserialize_as(reader)
    }
}

pub struct Same;

impl<T> BorshSerializeAs<T> for Same
where
    T: BorshSerialize,
{
    #[inline]
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        source.serialize(writer)
    }
}

impl<T> BorshDeserializeAs<T> for Same
where
    T: BorshDeserialize,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<T>
    where
        R: io::Read,
    {
        T::deserialize_reader(reader)
    }
}
