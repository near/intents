use crate::adapters::{BorshDeserializeAs, BorshSerializeAs};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize, io};
use std::{fmt::Display, marker::PhantomData, time::Duration};

pub struct DurationSeconds<I = u64>(PhantomData<I>);

impl<I> BorshSerializeAs<Duration> for DurationSeconds<I>
where
    I: TryFrom<u64> + BorshSerialize,
    I::Error: Display,
{
    fn serialize_as<W>(source: &Duration, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        I::try_from(source.as_secs())
            .map_err(|err| io::Error::other(err.to_string()))?
            .serialize(writer)
    }
}

impl<I> BorshDeserializeAs<Duration> for DurationSeconds<I>
where
    I: TryInto<u64> + BorshDeserialize,
    I::Error: Display,
{
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Duration>
    where
        R: io::Read,
    {
        I::deserialize_reader(reader)?
            .try_into()
            .map(Duration::from_secs)
            .map_err(|err| io::Error::other(err.to_string()))
    }
}

pub struct DurationMilliSeconds<I = u64>(PhantomData<I>);

impl<I> BorshSerializeAs<Duration> for DurationMilliSeconds<I>
where
    I: TryFrom<u128> + BorshSerialize,
    I::Error: Display,
{
    fn serialize_as<W>(source: &Duration, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        I::try_from(source.as_millis())
            .map_err(|err| io::Error::other(err.to_string()))?
            .serialize(writer)
    }
}

impl<I> BorshDeserializeAs<Duration> for DurationMilliSeconds<I>
where
    I: TryInto<u64> + BorshDeserialize,
    I::Error: Display,
{
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Duration>
    where
        R: io::Read,
    {
        I::deserialize_reader(reader)?
            .try_into()
            .map(Duration::from_millis)
            .map_err(|err| io::Error::other(err.to_string()))
    }
}

pub struct DurationMicroSeconds<I = u64>(PhantomData<I>);

impl<I> BorshSerializeAs<Duration> for DurationMicroSeconds<I>
where
    I: TryFrom<u128> + BorshSerialize,
    I::Error: Display,
{
    fn serialize_as<W>(source: &Duration, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        I::try_from(source.as_micros())
            .map_err(|err| io::Error::other(err.to_string()))?
            .serialize(writer)
    }
}

impl<I> BorshDeserializeAs<Duration> for DurationMicroSeconds<I>
where
    I: TryInto<u64> + BorshDeserialize,
    I::Error: Display,
{
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Duration>
    where
        R: io::Read,
    {
        I::deserialize_reader(reader)?
            .try_into()
            .map(Duration::from_micros)
            .map_err(|err| io::Error::other(err.to_string()))
    }
}

pub struct DurationNanoSeconds<I = u64>(PhantomData<I>);

impl<I> BorshSerializeAs<Duration> for DurationNanoSeconds<I>
where
    I: TryFrom<u128> + BorshSerialize,
    I::Error: Display,
{
    fn serialize_as<W>(source: &Duration, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        I::try_from(source.as_nanos())
            .map_err(|err| io::Error::other(err.to_string()))?
            .serialize(writer)
    }
}

impl<I> BorshDeserializeAs<Duration> for DurationNanoSeconds<I>
where
    I: TryInto<u64> + BorshDeserialize,
    I::Error: Display,
{
    fn deserialize_as<R>(reader: &mut R) -> io::Result<Duration>
    where
        R: io::Read,
    {
        I::deserialize_reader(reader)?
            .try_into()
            .map(Duration::from_nanos)
            .map_err(|err| io::Error::other(err.to_string()))
    }
}

#[cfg(feature = "abi")]
const _: () = {
    use crate::adapters::schema::impl_borsh_schema_as;

    impl_borsh_schema_as!(Duration, DurationSeconds);
    impl_borsh_schema_as!(Duration, DurationMilliSeconds);
    impl_borsh_schema_as!(Duration, DurationMicroSeconds);
    impl_borsh_schema_as!(Duration, DurationNanoSeconds);
};
