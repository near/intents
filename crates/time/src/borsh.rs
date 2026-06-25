use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters::{
    BorshDeserializeAs, BorshSerializeAs, TimestampMicroSeconds, TimestampMilliSeconds,
    TimestampNanoSeconds, TimestampSeconds,
};

use crate::Timestamp;

macro_rules! impl_borsh_serde_as {
    ($($a:ident,)+) => {
        const _: () = {
            $(
                impl<I> BorshSerializeAs<Timestamp> for $a<I>
                where
                    $a<I>: BorshSerializeAs<DateTime<Utc>>,
                {
                    fn serialize_as<W>(source: &Timestamp, writer: &mut W) -> std::io::Result<()>
                    where
                        W: std::io::Write,
                    {
                        Self::serialize_as(&source.0, writer)
                    }
                }

                impl<I> BorshDeserializeAs<Timestamp> for $a<I>
                where
                    $a<I>: BorshDeserializeAs<DateTime<Utc>>,
                {
                    fn deserialize_as<R>(reader: &mut R) -> std::io::Result<Timestamp>
                    where
                        R: std::io::Read,
                    {
                        Self::deserialize_as(reader).map(Timestamp)
                    }
                }
            )*
        };
    };
}

impl_borsh_serde_as! {
    TimestampSeconds, TimestampMilliSeconds, TimestampMicroSeconds, TimestampNanoSeconds,
}

#[cfg(test)]
mod tests {
    #[test]
    fn schema_as_usage() {
        use super::*;
        use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
        use chrono::TimeZone;
        use defuse_borsh_utils::adapters::{As, TimestampNanoSeconds};

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
            pub deadline: Timestamp,
        }

        let val = S {
            deadline: Timestamp::new(Utc.timestamp_opt(1_600_000_000, 123_456_789).unwrap()),
        };
        let bytes = borsh::to_vec(&val).unwrap();
        let decoded = S::try_from_slice(&bytes).unwrap();
        assert_eq!(val.deadline, decoded.deadline);
    }
}
