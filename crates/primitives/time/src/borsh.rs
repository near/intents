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

        #[cfg(feature = "abi")]
        const _: () = {
            use borsh::{BorshSchema, schema::{Declaration, Definition}};
            use defuse_borsh_utils::adapters::BorshSchemaAs;

            impl<I> BorshSchemaAs<Timestamp> for $name<I>
            where
                I: BorshSchema,
            {
                fn declaration_as() -> borsh::schema::Declaration {
                    <I as BorshSchema>::declaration()
                }

                fn add_definitions_recursively_as(
                    definitions: &mut std::collections::BTreeMap<
                        Declaration,
                        Definition,
                    >,
                ) {
                    <I as BorshSchema>::add_definitions_recursively(definitions);
                }
            }
        };
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
