use std::{fmt::Display, io, marker::PhantomData};

use borsh::{BorshDeserialize, BorshSerialize};
use defuse_borsh_utils::adapters::{BorshDeserializeAs, BorshSerializeAs};

use crate::Timestamp;

macro_rules! borsh_as {
    ($($vis:vis struct $name:ident: $int:ty {
        $as:ident,
        $from:ident,
    })*) => {$(
        $vis struct $name<I>(PhantomData<I>);

        impl<I> BorshSerializeAs<Timestamp> for $name<I>
        where
            I: TryFrom<$int> + BorshSerialize,
            I::Error: Display,
        {
            #[inline]
            fn serialize_as<W>(source: &Timestamp, writer: &mut W) -> io::Result<()>
            where
                W: io::Write,
            {
                let timestamp: $int = source.0.$as();
                I::try_from(timestamp)
                    .map_err(|err| io::Error::other(err.to_string()))?
                    .serialize(writer)
            }
        }

        impl<I> BorshDeserializeAs<Timestamp> for $name<I>
        where
            I: TryInto<$int> + BorshDeserialize,
            I::Error: Display,
        {
            fn deserialize_as<R>(reader: &mut R) -> io::Result<Timestamp>
            where
                R: io::Read,
            {
                let timestamp: $int = I::deserialize_reader(reader)?
                    .try_into()
                    .map_err(|err| io::Error::other(err.to_string()))?;
                jiff::Timestamp::$from(timestamp).map(Timestamp).map_err(io::Error::other)
            }
        }
    )*};
}

borsh_as! {
    pub struct TimestampSeconds: i64 {
        as_second,
        from_second,
    }

    pub struct TimestampMilliSeconds: i64 {
        as_millisecond,
        from_millisecond,
    }

    pub struct TimestampMicroSeconds: i64 {
        as_microsecond,
        from_microsecond,
    }

    pub struct TimestampNanoSeconds: i128 {
        as_nanosecond,
        from_nanosecond,
    }
}

// pub struct TimestampSeconds<I = i64>(PhantomData<I>);

// impl<I> BorshSerializeAs<Timestamp> for TimestampSeconds<I>
// where
//     I: TryFrom<i64> + BorshSerialize,
//     I::Error: Display,
// {
//     #[inline]
//     fn serialize_as<W>(source: &Timestamp, writer: &mut W) -> io::Result<()>
//     where
//         W: io::Write,
//     {
//         // TODO
//         I::try_from(source.as_second())
//             .map_err(|err| io::Error::other(err.to_string()))?
//             .serialize(writer)
//     }
// }

// impl<I> BorshDeserializeAs<Timestamp> for TimestampSeconds<I>
// where
//     I: TryInto<i64> + BorshDeserialize,
//     I::Error: Display,
// {
//     fn deserialize_as<R>(reader: &mut R) -> io::Result<Timestamp>
//     where
//         R: io::Read,
//     {
//         let timestamp: i64 = I::deserialize_reader(reader)?
//             .try_into()
//             .map_err(|err| io::Error::other(err.to_string()))?;
//         Timestamp::from_second(timestamp).map_err(io::Error::other)
//     }
// }

// pub struct TimestampMilliSeconds<I = i64>(PhantomData<I>);

// impl<I> BorshSerializeAs<Timestamp> for TimestampMilliSeconds<I>
// where
//     I: TryFrom<i64> + BorshSerialize,
//     I::Error: Display,
// {
//     #[inline]
//     fn serialize_as<W>(source: &Timestamp, writer: &mut W) -> io::Result<()>
//     where
//         W: io::Write,
//     {
//         I::try_from(source.as_millisecond())
//             .map_err(|err| io::Error::other(err.to_string()))?
//             .serialize(writer)
//     }
// }

// impl<I> BorshDeserializeAs<Timestamp> for TimestampMilliSeconds<I>
// where
//     I: TryInto<i64> + BorshDeserialize,
//     I::Error: Display,
// {
//     fn deserialize_as<R>(reader: &mut R) -> io::Result<Timestamp>
//     where
//         R: io::Read,
//     {
//         let timestamp: i64 = I::deserialize_reader(reader)?
//             .try_into()
//             .map_err(|err| io::Error::other(err.to_string()))?;
//         Timestamp::from_millisecond(timestamp).map_err(io::Error::other)
//     }
// }

// pub struct TimestampMicroSeconds<I = i64>(PhantomData<I>);

// impl<I> BorshSerializeAs<Timestamp> for TimestampMicroSeconds<I>
// where
//     I: TryFrom<i64> + BorshSerialize,
//     I::Error: Display,
// {
//     #[inline]
//     fn serialize_as<W>(source: &Timestamp, writer: &mut W) -> io::Result<()>
//     where
//         W: io::Write,
//     {
//         I::try_from(source.as_microsecond())
//             .map_err(|err| io::Error::other(err.to_string()))?
//             .serialize(writer)
//     }
// }

// impl<I> BorshDeserializeAs<Timestamp> for TimestampMicroSeconds<I>
// where
//     I: TryInto<i64> + BorshDeserialize,
//     I::Error: Display,
// {
//     fn deserialize_as<R>(reader: &mut R) -> io::Result<Timestamp>
//     where
//         R: io::Read,
//     {
//         let timestamp: i64 = I::deserialize_reader(reader)?
//             .try_into()
//             .map_err(|err| io::Error::other(err.to_string()))?;
//         Timestamp::from_microsecond(timestamp).map_err(io::Error::other)
//     }
// }

// pub struct TimestampNanoSeconds<I = i64>(PhantomData<I>);

// impl<I> BorshSerializeAs<Timestamp> for TimestampNanoSeconds<I>
// where
//     I: TryFrom<i128> + BorshSerialize,
//     I::Error: Display,
// {
//     #[inline]
//     fn serialize_as<W>(source: &Timestamp, writer: &mut W) -> io::Result<()>
//     where
//         W: io::Write,
//     {
//         I::try_from(source.as_nanosecond())
//             .map_err(|err| io::Error::other(err.to_string()))?
//             .serialize(writer)
//     }
// }

// impl<I> BorshDeserializeAs<Timestamp> for TimestampNanoSeconds<I>
// where
//     I: TryInto<i128> + BorshDeserialize,
//     I::Error: Display,
// {
//     fn deserialize_as<R>(reader: &mut R) -> io::Result<Timestamp>
//     where
//         R: io::Read,
//     {
//         let timestamp: i128 = I::deserialize_reader(reader)?
//             .try_into()
//             .map_err(|err| io::Error::other(err.to_string()))?;
//         Timestamp::from_nanosecond(timestamp).map_err(io::Error::other)
//     }
// }

// TODO
// #[cfg(feature = "abi")]
// const _: () = {
//     use crate::adapters::schema::impl_borsh_schema_as;

//     impl_borsh_schema_as!(Timestamp, TimestampSeconds);
//     impl_borsh_schema_as!(Timestamp, TimestampMilliSeconds);
//     impl_borsh_schema_as!(Timestamp, TimestampMicroSeconds);
//     impl_borsh_schema_as!(Timestamp, TimestampNanoSeconds);
// };

// TODO
// #[cfg(test)]
// mod tests {
//     use crate::adapters::tests::roundtrip_as;

//     use super::*;

//     #[test]
//     fn timestamp_seconds_i64_roundtrip() {
//         roundtrip_as::<_, TimestampSeconds<i64>>(&Timestamp::from_second(1_600_000_000).unwrap());
//     }

//     #[test]
//     fn timestamp_milliseconds_i64_roundtrip() {
//         roundtrip_as::<_, TimestampMilliSeconds<i64>>(
//             &Timestamp::from_millisecond(1_600_000_000_123).unwrap(),
//         );
//     }

//     #[test]
//     fn timestamp_microseconds_i64_roundtrip() {
//         roundtrip_as::<_, TimestampMicroSeconds<i64>>(
//             &Timestamp::from_microsecond(1_600_000_000_123_456).unwrap(),
//         );
//     }

//     #[test]
//     fn timestamp_nanoseconds_i64_roundtrip() {
//         roundtrip_as::<_, TimestampNanoSeconds<i64>>(
//             &Timestamp::from_nanosecond(1_600_000_000_123_456_789).unwrap(),
//         );
//     }
// }
