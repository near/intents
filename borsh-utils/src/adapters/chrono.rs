use std::{fmt::Display, io, marker::PhantomData};

use chrono::{DateTime, Utc};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

use crate::adapters::BorshDeserializeAs;

use super::BorshSerializeAs;

pub struct TimestampSeconds<I = i64>(PhantomData<I>);

impl<I> BorshSerializeAs<DateTime<Utc>> for TimestampSeconds<I>
where
    I: TryFrom<i64> + BorshSerialize,
    I::Error: Display,
{
    #[inline]
    fn serialize_as<W>(source: &DateTime<Utc>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        I::try_from(source.timestamp())
            .map_err(|err| io::Error::other(err.to_string()))?
            .serialize(writer)
    }
}

impl<I> BorshDeserializeAs<DateTime<Utc>> for TimestampSeconds<I>
where
    I: TryInto<i64> + BorshDeserialize,
    I::Error: Display,
{
    fn deserialize_as<R>(reader: &mut R) -> io::Result<DateTime<Utc>>
    where
        R: io::Read,
    {
        let timestamp = I::deserialize_reader(reader)?
            .try_into()
            .map_err(|err| io::Error::other(err.to_string()))?;
        DateTime::<Utc>::from_timestamp(timestamp, 0)
            .ok_or_else(|| io::Error::other("timestamp: out of range"))
    }
}

pub struct TimestampMilliSeconds<I = i64>(PhantomData<I>);

impl<I> BorshSerializeAs<DateTime<Utc>> for TimestampMilliSeconds<I>
where
    I: TryFrom<i64> + BorshSerialize,
    I::Error: Display,
{
    #[inline]
    fn serialize_as<W>(source: &DateTime<Utc>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        I::try_from(source.timestamp_millis())
            .map_err(|err| io::Error::other(err.to_string()))?
            .serialize(writer)
    }
}

impl<I> BorshDeserializeAs<DateTime<Utc>> for TimestampMilliSeconds<I>
where
    I: TryInto<i64> + BorshDeserialize,
    I::Error: Display,
{
    fn deserialize_as<R>(reader: &mut R) -> io::Result<DateTime<Utc>>
    where
        R: io::Read,
    {
        let timestamp = I::deserialize_reader(reader)?
            .try_into()
            .map_err(|err| io::Error::other(err.to_string()))?;
        DateTime::<Utc>::from_timestamp_millis(timestamp)
            .ok_or_else(|| io::Error::other("timestamp: out of range"))
    }
}

pub struct TimestampMicroSeconds<I = i64>(PhantomData<I>);

impl<I> BorshSerializeAs<DateTime<Utc>> for TimestampMicroSeconds<I>
where
    I: TryFrom<i64> + BorshSerialize,
    I::Error: Display,
{
    #[inline]
    fn serialize_as<W>(source: &DateTime<Utc>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        I::try_from(source.timestamp_micros())
            .map_err(|err| io::Error::other(err.to_string()))?
            .serialize(writer)
    }
}

impl<I> BorshDeserializeAs<DateTime<Utc>> for TimestampMicroSeconds<I>
where
    I: TryInto<i64> + BorshDeserialize,
    I::Error: Display,
{
    fn deserialize_as<R>(reader: &mut R) -> io::Result<DateTime<Utc>>
    where
        R: io::Read,
    {
        let timestamp = I::deserialize_reader(reader)?
            .try_into()
            .map_err(|err| io::Error::other(err.to_string()))?;
        DateTime::<Utc>::from_timestamp_micros(timestamp)
            .ok_or_else(|| io::Error::other("timestamp: out of range"))
    }
}

pub struct TimestampNanoSeconds<I = i64>(PhantomData<I>);

impl<I> BorshSerializeAs<DateTime<Utc>> for TimestampNanoSeconds<I>
where
    I: TryFrom<i64> + BorshSerialize,
    I::Error: Display,
{
    #[inline]
    fn serialize_as<W>(source: &DateTime<Utc>, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        I::try_from(
            source
                .timestamp_nanos_opt()
                .ok_or_else(|| io::Error::other("timestamp: out of range"))?,
        )
        .map_err(|err| io::Error::other(err.to_string()))?
        .serialize(writer)
    }
}

impl<I> BorshDeserializeAs<DateTime<Utc>> for TimestampNanoSeconds<I>
where
    I: TryInto<i64> + BorshDeserialize,
    I::Error: Display,
{
    fn deserialize_as<R>(reader: &mut R) -> io::Result<DateTime<Utc>>
    where
        R: io::Read,
    {
        let timestamp = I::deserialize_reader(reader)?
            .try_into()
            .map_err(|err| io::Error::other(err.to_string()))?;
        Ok(DateTime::<Utc>::from_timestamp_nanos(timestamp))
    }
}
