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
            #[inline]
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
                #[inline]
                fn declaration_as() -> borsh::schema::Declaration {
                    <I as BorshSchema>::declaration()
                }

                #[inline]
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

    pub struct TimestampMicroSeconds: i128 {
        as_micros,
        from_micros,
    }

    pub struct TimestampNanoSeconds: i128 {
        as_nanos,
        from_nanos,
    }
}

#[cfg(test)]
#[allow(clippy::inconsistent_digit_grouping)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use super::*;

    #[rstest]
    fn timestamp_secs_roundtrip<I>(
        #[values(
            0i64, 0u64, 0i32, 0u32,
            1_600_000_000i64, 1_600_000_000u64,
            1782395622i64, 1782395622u64,
            -1782395622i64, -1782395622i32,
        )]
        secs: I,
    ) where
        I: TryInto<i64, Error: Debug + Display>
            + TryFrom<i64, Error: Display>
            + BorshSerialize
            + BorshDeserialize,
    {
        roundtrip_as::<_, TimestampSeconds<I>>(
            &Timestamp::from_secs(secs.try_into().unwrap()).unwrap(),
        );
    }

    #[rstest]
    fn timestamp_millis_roundtrip<I>(
        #[values(
            0i64, 0u64, 0i32, 0u32,
            1_600_000_000i64, 1_600_000_000u64,
            1782395622_123i64, 1782395622_123u64,
            -1782395622_123i64
        )]
        millis: I,
    ) where
        I: TryInto<i64, Error: Debug + Display>
            + TryFrom<i64, Error: Display>
            + BorshSerialize
            + BorshDeserialize,
    {
        roundtrip_as::<_, TimestampMilliSeconds<I>>(
            &Timestamp::from_millis(millis.try_into().unwrap()).unwrap(),
        );
    }

    #[rstest]
    fn timestamp_micros_roundtrip<I>(
        #[values(
            0i128, 0u128, 0i64, 0u64, 0i32, 0u32,
            1_600_000_000i128, 1_600_000_000u128,
            1_600_000_000i64, 1_600_000_000u64,
            1782395622_123456i128, 1782395622_123456u128,
            -1782395622_123456i128, 1782395622_123456i64,
            -1782395622_123456i64, 1782395622_123456u64,
        )]
        micros: I,
    ) where
        I: TryInto<i128, Error: Debug + Display>
            + TryFrom<i128, Error: Display>
            + BorshSerialize
            + BorshDeserialize,
    {
        roundtrip_as::<_, TimestampMicroSeconds<I>>(
            &Timestamp::from_micros(micros.try_into().unwrap()).unwrap(),
        );
    }

    #[rstest]
    fn timestamp_nanos_roundtrip<I>(
        #[values(
            0i128, 0u128, 0i64, 0u64, 0i32, 0u32,
            1_600_000_000i128, 1_600_000_000u128,
            1_600_000_000i64, 1_600_000_000u64,
            1782395622_123456789i128, 1782395622_123456789u128,
            -1782395622_123456789i128, 1782395622_123456789i64,
            -1782395622_123456789i64, 1782395622_123456789u64,
        )]
        nanos: I,
    ) where
        I: TryInto<i128, Error: Debug + Display>
            + TryFrom<i128, Error: Display>
            + BorshSerialize
            + BorshDeserialize,
    {
        roundtrip_as::<_, TimestampNanoSeconds<I>>(
            &Timestamp::from_nanos(nanos.try_into().unwrap()).unwrap(),
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
