use std::{fmt::Display, io, marker::PhantomData};

use borsh::{BorshDeserialize, BorshSerialize};
use defuse_borsh_utils::{BorshDeserializeAs, BorshSerializeAs};

use crate::{Overflow, Timestamp};

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
                let timestamp: $int = source.$as();
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
                Timestamp::$from(timestamp).ok_or(Overflow).map_err( io::Error::other)
            }
        }

        #[cfg(feature = "abi")]
        const _: () = {
            use borsh::{BorshSchema, schema::{Declaration, Definition}};
            use defuse_borsh_utils::BorshSchemaAs;

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
        as_secs,
        from_secs,
    }

    pub struct TimestampMilliSeconds: i64 {
        as_millis,
        from_millis,
    }

    pub struct TimestampMicroSeconds: i64 {
        as_micros,
        from_micros,
    }

    pub struct TimestampNanoSeconds: i128 {
        as_nanos,
        from_nanos,
    }
}

// TODO
#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use super::*;

    #[test]
    fn timestamp_seconds_i64_roundtrip() {
        roundtrip_as::<_, TimestampSeconds<i64>>(&Timestamp::from_secs(1_600_000_000).unwrap());
    }

    #[test]
    fn timestamp_milliseconds_i64_roundtrip() {
        roundtrip_as::<_, TimestampMilliSeconds<i64>>(
            &Timestamp::from_millis(1_600_000_000_123).unwrap(),
        );
    }

    #[test]
    fn timestamp_microseconds_i64_roundtrip() {
        roundtrip_as::<_, TimestampMicroSeconds<i64>>(
            &Timestamp::from_micros(1_600_000_000_123_456).unwrap(),
        );
    }

    #[test]
    fn timestamp_nanoseconds_i64_roundtrip() {
        roundtrip_as::<_, TimestampNanoSeconds<i64>>(
            &Timestamp::from_nanos(1_600_000_000_123_456_789).unwrap(),
        );
    }

    // Helper roundtrip
    #[track_caller]
    fn roundtrip_as<T, As>(orig: &T)
    where
        As: BorshSerializeAs<T> + BorshDeserializeAs<T>,
        T: PartialEq + Debug,
    {
        let mut buf = Vec::new();
        As::serialize_as(orig, &mut buf).expect("serialize_as");
        let deserialized = As::deserialize_as(&mut buf.as_slice()).expect("deserialize_as");
        assert_eq!(
            &deserialized, orig,
            "deserialized value differs from the original one"
        );
    }

    #[test]
    fn schema_as_usage() {
        use borsh::BorshSchema;
        use defuse_borsh_utils::As;

        #[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
        struct S {
            #[borsh(
                serialize_with = "As::<TimestampNanoSeconds<i64>>::serialize",
                deserialize_with = "As::<TimestampNanoSeconds<i64>>::deserialize",
                schema(with_funcs(
                    declaration = "As::<TimestampNanoSeconds<i64>>::declaration",
                    definitions = "As::<TimestampNanoSeconds<i64>>::add_definitions_recursively",
                ))
            )]
            pub deadline: Timestamp,
        }

        let val = S {
            deadline: Timestamp::from_nanos(1_600_000_000_123_456_789).unwrap(),
        };
        let bytes = borsh::to_vec(&val).unwrap();
        let decoded = S::try_from_slice(&bytes).unwrap();
        assert_eq!(val.deadline, decoded.deadline);
    }
}
