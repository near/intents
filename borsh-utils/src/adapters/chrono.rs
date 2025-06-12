use super::BorshSerializeAs;
use crate::adapters::BorshDeserializeAs;
use chrono::{DateTime, Utc};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use std::{fmt::Display, io, marker::PhantomData};

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};
    use std::io::Cursor;

    #[test]
    fn timestamp_seconds_i64_roundtrip() {
        let dt: DateTime<Utc> = Utc.timestamp_opt(1_600_000_000, 0).unwrap();
        let mut buf = Vec::new();
        TimestampSeconds::<i64>::serialize_as(&dt, &mut buf).unwrap();
        let mut cursor = Cursor::new(&buf);
        let dt2 = TimestampSeconds::<i64>::deserialize_as(&mut cursor).unwrap();
        assert_eq!(dt, dt2);
    }

    #[test]
    fn timestamp_milliseconds_i64_roundtrip() {
        let millis = 1_600_000_000_123;
        let dt: DateTime<Utc> = DateTime::<Utc>::from_timestamp_millis(millis).unwrap();
        let mut buf = Vec::new();
        TimestampMilliSeconds::<i64>::serialize_as(&dt, &mut buf).unwrap();
        let mut cursor = Cursor::new(&buf);
        let dt2 = TimestampMilliSeconds::<i64>::deserialize_as(&mut cursor).unwrap();
        assert_eq!(dt, dt2);
    }

    #[test]
    fn timestamp_microseconds_i64_roundtrip() {
        let micros = 1_600_000_000_123_456;
        let dt: DateTime<Utc> = DateTime::<Utc>::from_timestamp_micros(micros).unwrap();
        let mut buf = Vec::new();
        TimestampMicroSeconds::<i64>::serialize_as(&dt, &mut buf).unwrap();
        let mut cursor = Cursor::new(&buf);
        let dt2 = TimestampMicroSeconds::<i64>::deserialize_as(&mut cursor).unwrap();
        assert_eq!(dt, dt2);
    }

    #[test]
    fn timestamp_nanoseconds_i64_roundtrip() {
        let nanos = 1_600_000_000_123_456_789;
        let dt: DateTime<Utc> = DateTime::<Utc>::from_timestamp_nanos(nanos);
        let mut buf = Vec::new();
        TimestampNanoSeconds::<i64>::serialize_as(&dt, &mut buf).unwrap();
        let mut cursor = Cursor::new(&buf);
        let dt2 = TimestampNanoSeconds::<i64>::deserialize_as(&mut cursor).unwrap();
        assert_eq!(dt, dt2);
    }

    #[test]
    fn invalid_timestamp_seconds_overflow() {
        let dt: DateTime<Utc> = Utc.timestamp_opt(127, 0).unwrap();
        let mut buf = Vec::new();
        let res = TimestampSeconds::<i8>::serialize_as(&dt, &mut buf);
        assert!(res.is_err());
    }

    #[test]
    fn invalid_timestamp_nanoseconds_out_of_range() {
        let invalid_nanos = i128::from(i64::MAX) * 1_000_000_000;
        let buf = invalid_nanos.to_le_bytes();
        let mut cursor = Cursor::new(&buf[..]);
        let res = TimestampNanoSeconds::<i64>::deserialize_as(&mut cursor);
        assert!(res.is_err());
    }
}
