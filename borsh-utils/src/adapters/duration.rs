use std::{fmt::Display, marker::PhantomData, time::Duration};

use near_sdk::borsh::{BorshDeserialize, BorshSerialize, io};

use crate::adapters::{BorshDeserializeAs, BorshSerializeAs};

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
    use std::collections::BTreeMap;

    use near_sdk::borsh::{
        BorshSchema,
        schema::{Declaration, Definition},
    };

    use crate::adapters::BorshSchemaAs;

    macro_rules! impl_borsh_schema_as {
        ($($ts:ident),*) => {$(
            impl<I> BorshSchemaAs<Duration> for $ts<I>
            where
                I: BorshSchema,
            {
                fn add_definitions_recursively_as(definitions: &mut BTreeMap<Declaration, Definition>) {
                    I::add_definitions_recursively(definitions);
                }

                fn declaration_as() -> Declaration {
                    I::declaration()
                }
            })*
        };

    }

    impl_borsh_schema_as!(
        DurationSeconds,
        DurationMilliSeconds,
        DurationMicroSeconds,
        DurationNanoSeconds
    );
};
