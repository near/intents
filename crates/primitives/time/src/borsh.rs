use defuse_borsh_utils::adapters::{
    BorshDeserializeAs, BorshSerializeAs, TimestampMicroSeconds, TimestampMilliSeconds,
    TimestampNanoSeconds, TimestampSeconds,
};

macro_rules! impl_borsh_serde_as {
    ($($a:ident,)+) => {$(
        impl<I> BorshSerializeAs<DateTime> for $a<I>
        where
            $a<I>: BorshSerializeAs<::chrono::DateTime<Utc>>,
        {
            fn serialize_as<W>(source: &DateTime, writer: &mut W) -> std::io::Result<()>
            where
                W: std::io::Write,
            {
                Self::serialize_as(&source.0, writer)
            }
        }

        impl<I> BorshDeserializeAs<DateTime> for $a<I>
        where
            $a<I>: BorshDeserializeAs<::chrono::DateTime<Utc>>,
        {
            fn deserialize_as<R>(reader: &mut R) -> std::io::Result<DateTime>
            where
                R: std::io::Read,
            {
                Self::deserialize_as(reader).map(DateTime)
            }
        }
    )*};
}

impl_borsh_serde_as! {
    TimestampSeconds,
    TimestampMilliSeconds,
    TimestampMicroSeconds,
    TimestampNanoSeconds,
}

#[cfg(test)]
mod tests {
    use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
    use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};

    use super::*;

    #[test]
    fn borsh_schema_as_usage() {
        #[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
        struct S {
            #[borsh(
                serialize_with = "As::<TimestampNanoSeconds>::serialize",
                deserialize_with = "As::<TimestampNanoSeconds>::deserialize",
                schema(with_funcs(
                    declaration = "As::<TimestampNanoSeconds>::declaration",
                    definitions = "As::<TimestampNanoSeconds>::add_definitions_recursively",
                ))
            )]
            pub deadline: DateTime,
        }

        let val = S {
            deadline: Utc
                .timestamp_opt(1_600_000_000, 123_456_789)
                .unwrap()
                .into(),
        };
        let bytes = borsh::to_vec(&val).unwrap();
        let decoded = S::try_from_slice(&bytes).unwrap();
        assert_eq!(val.deadline, decoded.deadline);
    }
}
