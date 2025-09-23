//! Analog of [serde_with](https://docs.rs/serde_with) for [borsh](https://docs.rs/borsh)

use std::{
    fmt::{self, Display},
    io::{self, Read},
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
};

use defuse_io_utils::ReadExt;
use impl_tools::autoimpl;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};

#[cfg(feature = "chrono")]
mod chrono;
#[cfg(feature = "chrono")]
pub use self::chrono::*;

pub trait BorshSerializeAs<T: ?Sized> {
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write;
}

pub trait BorshDeserializeAs<T> {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<T>
    where
        R: io::Read;
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

/// Analog for [`serde_with::Same`](https://docs.rs/serde_with/latest/serde_with/struct.Same.html)
#[derive(Debug, Eq, PartialEq)]
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

#[autoimpl(Deref using self.value)]
#[autoimpl(DerefMut using self.value)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AsWrap<T, As: ?Sized> {
    value: T,
    _marker: PhantomData<As>,
}

impl<T, As: ?Sized> AsWrap<T, As> {
    #[must_use]
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }

    /// Return the inner value of type `T`.
    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T, As: ?Sized> From<T> for AsWrap<T, As> {
    #[inline]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

pub fn to_vec_as<T, As>(source: &T) -> io::Result<Vec<u8>>
where
    As: BorshSerializeAs<T> + ?Sized,
{
    borsh::to_vec(&AsWrap::<&T, &As>::new(source))
}

impl<T, As> BorshDeserialize for AsWrap<T, As>
where
    As: BorshDeserializeAs<T> + ?Sized,
{
    #[inline]
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        As::deserialize_as(reader).map(Self::new)
    }
}

impl<T, As> BorshSerialize for AsWrap<T, As>
where
    As: BorshSerializeAs<T> + ?Sized,
{
    #[inline]
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        As::serialize_as(&self.value, writer)
    }
}

impl<T, As> fmt::Debug for AsWrap<T, As>
where
    T: fmt::Debug,
    As: ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.value, f)
    }
}

impl<T, As> fmt::Display for AsWrap<T, As>
where
    T: fmt::Display,
    As: ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl<T, As> BorshSerializeAs<&T> for &As
where
    T: ?Sized,
    As: BorshSerializeAs<T> + ?Sized,
{
    #[inline]
    fn serialize_as<W>(source: &&T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        As::serialize_as(source, writer)
    }
}

impl<T, As> BorshSerializeAs<&mut T> for &mut As
where
    T: ?Sized,
    As: BorshSerializeAs<T> + ?Sized,
{
    #[inline]
    fn serialize_as<W>(source: &&mut T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        As::serialize_as(source, writer)
    }
}

impl<T, As> BorshSerializeAs<Option<T>> for Option<As>
where
    As: BorshSerializeAs<T>,
{
    #[inline]
    fn serialize_as<W>(source: &Option<T>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        source
            .as_ref()
            .map(AsWrap::<&T, &As>::new)
            .serialize(writer)
    }
}

impl<T, As> BorshDeserializeAs<Option<T>> for Option<As>
where
    As: BorshDeserializeAs<T>,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Option<T>>
    where
        R: io::Read,
    {
        Ok(Option::<AsWrap<T, As>>::deserialize_reader(reader)?.map(AsWrap::into_inner))
    }
}

impl<T, As> BorshSerializeAs<Box<T>> for Box<As>
where
    As: BorshSerializeAs<T> + ?Sized,
{
    #[inline]
    fn serialize_as<W>(source: &Box<T>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        AsWrap::<&T, &As>::new(source).serialize(writer)
    }
}

impl<T, As> BorshDeserializeAs<Box<T>> for Box<As>
where
    As: BorshDeserializeAs<T> + ?Sized,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Box<T>>
    where
        R: io::Read,
    {
        AsWrap::<T, As>::deserialize_reader(reader)
            .map(AsWrap::into_inner)
            .map(Box::new)
    }
}

impl<T, As> BorshSerializeAs<Rc<T>> for Rc<As>
where
    As: BorshSerializeAs<T> + ?Sized,
{
    #[inline]
    fn serialize_as<W>(source: &Rc<T>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        AsWrap::<&T, &As>::new(source).serialize(writer)
    }
}

impl<T, As> BorshDeserializeAs<Rc<T>> for Rc<As>
where
    As: BorshDeserializeAs<T> + ?Sized,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Rc<T>>
    where
        R: io::Read,
    {
        AsWrap::<T, As>::deserialize_reader(reader)
            .map(AsWrap::into_inner)
            .map(Rc::new)
    }
}

impl<T, As> BorshSerializeAs<Arc<T>> for Arc<As>
where
    As: BorshSerializeAs<T> + ?Sized,
{
    #[inline]
    fn serialize_as<W>(source: &Arc<T>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        AsWrap::<&T, &As>::new(source).serialize(writer)
    }
}

impl<T, As> BorshDeserializeAs<Arc<T>> for Arc<As>
where
    As: BorshDeserializeAs<T> + ?Sized,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Arc<T>>
    where
        R: io::Read,
    {
        AsWrap::<T, As>::deserialize_reader(reader)
            .map(AsWrap::into_inner)
            .map(Arc::new)
    }
}

impl<T, As> BorshSerializeAs<[T]> for [As]
where
    As: BorshSerializeAs<T>,
{
    #[inline]
    fn serialize_as<W>(source: &[T], writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        source.iter().try_for_each(|v| As::serialize_as(v, writer))
    }
}

impl<T, As, const N: usize> BorshSerializeAs<[T; N]> for [As; N]
where
    As: BorshSerializeAs<T>,
{
    #[inline]
    fn serialize_as<W>(source: &[T; N], writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        <&[As]>::serialize_as(&source.as_slice(), writer)
    }
}

impl<T, As, const N: usize> BorshDeserializeAs<[T; N]> for [As; N]
where
    As: BorshDeserializeAs<T>,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<[T; N]>
    where
        R: io::Read,
    {
        // TODO: replace with [`core::array::try_from_fn`](https://github.com/rust-lang/rust/issues/89379) when stabilized
        array_util::try_from_fn(|_i| As::deserialize_as(reader))
    }
}

macro_rules! impl_borsh_serde_as_for_tuple {
    ($($n:tt:$t:ident as $a:ident),+) => {
        impl<$($t, $a),+> BorshSerializeAs<($($t,)+)> for ($($a,)+)
        where $(
            $a: BorshSerializeAs<$t>,
        )+
        {
            #[inline]
            fn serialize_as<W>(source: &($($t,)+), writer: &mut W) -> io::Result<()>
            where
                W: io::Write,
            {
                $(
                    $a::serialize_as(&source.$n, writer)?;
                )+
                Ok(())
            }
        }

        impl<$($t, $a),+> BorshDeserializeAs<($($t,)+)> for ($($a,)+)
        where $(
            $a: BorshDeserializeAs<$t>,
        )+
        {
            #[inline]
            fn deserialize_as<R>(reader: &mut R) -> io::Result<($($t,)+)>
            where
                R: io::Read,
            {
                Ok(($(
                    $a::deserialize_as(reader)?,
                )+))
            }
        }
    };
}
impl_borsh_serde_as_for_tuple!(0:T0 as As0);
impl_borsh_serde_as_for_tuple!(0:T0 as As0,1:T1 as As1);
impl_borsh_serde_as_for_tuple!(0:T0 as As0,1:T1 as As1,2:T2 as As2);
impl_borsh_serde_as_for_tuple!(0:T0 as As0,1:T1 as As1,2:T2 as As2,3:T3 as As3);
impl_borsh_serde_as_for_tuple!(0:T0 as As0,1:T1 as As1,2:T2 as As2,3:T3 as As3,4:T4 as As4);
impl_borsh_serde_as_for_tuple!(0:T0 as As0,1:T1 as As1,2:T2 as As2,3:T3 as As3,4:T4 as As4,5:T5 as As5);
impl_borsh_serde_as_for_tuple!(0:T0 as As0,1:T1 as As1,2:T2 as As2,3:T3 as As3,4:T4 as As4,5:T5 as As5,6:T6 as As6);
impl_borsh_serde_as_for_tuple!(0:T0 as As0,1:T1 as As1,2:T2 as As2,3:T3 as As3,4:T4 as As4,5:T5 as As5,6:T6 as As6,7:T7 as As7);
impl_borsh_serde_as_for_tuple!(0:T0 as As0,1:T1 as As1,2:T2 as As2,3:T3 as As3,4:T4 as As4,5:T5 as As5,6:T6 as As6,7:T7 as As7,8:T8 as As8);
impl_borsh_serde_as_for_tuple!(0:T0 as As0,1:T1 as As1,2:T2 as As2,3:T3 as As3,4:T4 as As4,5:T5 as As5,6:T6 as As6,7:T7 as As7,8:T8 as As8,9:T9 as As9);

pub struct FromInto<T: ?Sized>(PhantomData<T>);

impl<T, U> BorshSerializeAs<T> for FromInto<U>
where
    T: Into<U> + Clone,
    U: BorshSerialize,
{
    #[inline]
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        source.clone().into().serialize(writer)
    }
}

impl<T, U> BorshDeserializeAs<T> for FromInto<U>
where
    U: BorshDeserialize + Into<T>,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<T>
    where
        R: io::Read,
    {
        U::deserialize_reader(reader).map(Into::into)
    }
}

pub struct FromIntoRef<T: ?Sized>(PhantomData<T>);

impl<T, U> BorshSerializeAs<T> for FromIntoRef<U>
where
    for<'a> &'a T: Into<U>,
    U: BorshSerialize,
{
    #[inline]
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        source.into().serialize(writer)
    }
}

impl<T, U> BorshDeserializeAs<T> for FromIntoRef<U>
where
    U: BorshDeserialize + Into<T>,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<T>
    where
        R: io::Read,
    {
        U::deserialize_reader(reader).map(Into::into)
    }
}

pub struct TryFromInto<T: ?Sized>(PhantomData<T>);

impl<T, U> BorshSerializeAs<T> for TryFromInto<U>
where
    T: TryInto<U> + Clone,
    <T as TryInto<U>>::Error: Display,
    U: BorshSerialize,
{
    #[inline]
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        source
            .clone()
            .try_into()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?
            .serialize(writer)
    }
}

impl<T, U> BorshDeserializeAs<T> for TryFromInto<U>
where
    U: BorshDeserialize + TryInto<T>,
    <U as TryInto<T>>::Error: Display,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<T>
    where
        R: io::Read,
    {
        U::deserialize_reader(reader).and_then(|v| {
            v.try_into()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
        })
    }
}

pub struct Or<T1: ?Sized, T2: ?Sized>(PhantomData<T1>, PhantomData<T2>);

impl<T, As1, As2> BorshDeserializeAs<T> for Or<As1, As2>
where
    As1: BorshDeserializeAs<T> + ?Sized,
    As2: BorshDeserializeAs<T> + ?Sized,
{
    #[inline]
    fn deserialize_as<R>(reader: &mut R) -> io::Result<T>
    where
        R: io::Read,
    {
        let mut buf = Vec::new();
        As1::deserialize_as(&mut reader.tee(&mut buf))
            .or_else(|_| As2::deserialize_as(&mut buf.chain(reader)))
    }
}

#[cfg(test)]
mod tests;
