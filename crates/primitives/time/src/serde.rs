use core::marker::PhantomData;

use serde::{Deserializer, Serializer, de};
use serde_with::{DeserializeAs, Same, SerializeAs};

use crate::{Overflow, Timestamp};

macro_rules! serde_as {
    ($($vis:vis struct $name:ident: $int:ty {
        $as:ident,
        $from:ident,
    })*) => {$(
        $vis struct $name<F: ?Sized = Same>(PhantomData<F>);

        impl<F> SerializeAs<Timestamp> for $name<F>
        where
            F: SerializeAs<$int> + ?Sized,
        {
            fn serialize_as<S>(source: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let timestamp: $int = source.$as();
                <F as SerializeAs<$int>>::serialize_as(&timestamp, serializer)
            }
        }

        impl<'de, F> DeserializeAs<'de, Timestamp> for $name<F>
        where
            F: DeserializeAs<'de, $int> + ?Sized,
        {
            fn deserialize_as<D>(deserializer: D) -> Result<Timestamp, D::Error>
            where
                D: Deserializer<'de>,
            {
                let timestamp = <F as DeserializeAs<'de, $int>>::deserialize_as(deserializer)?;
                Timestamp::$from(timestamp).ok_or(Overflow).map_err( de::Error::custom)
            }
        }

        #[cfg(feature = "schemars-v0_8")]
        const _: () = {
            use schemars::{SchemaGenerator, schema::Schema};
            use serde_with::schemars_0_8::JsonSchemaAs;

            impl<F> JsonSchemaAs<Timestamp> for $name<F>
            where
                F: JsonSchemaAs<$int> + ?Sized,
            {
                #[inline]
                fn is_referenceable() -> bool {
                    false
                }

                #[inline]
                fn schema_name() -> String {
                    stringify!($name<F>).into()
                }

                #[inline]
                fn json_schema(generator: &mut SchemaGenerator) -> Schema {
                    <F as JsonSchemaAs<$int>>::json_schema(generator)
                }
            }
        };
    )*};
}

serde_as! {
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

#[cfg(feature = "schemars-v0_8")]
const _: () = {
    use schemars::{JsonSchema, SchemaGenerator, schema::Schema};

    impl JsonSchema for Timestamp {
        fn is_referenceable() -> bool {
            true
        }

        fn schema_name() -> String {
            stringify!(Timestamp).to_string()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            let mut schema = String::json_schema(generator).into_object();
            schema.metadata().examples = [
                Self::UNIX_EPOCH,
                #[allow(clippy::inconsistent_digit_grouping)]
                Self::from_nanos(1782395622_123456789).unwrap(),
            ]
            .iter()
            .map(serde_json::to_value)
            .map(Result::unwrap)
            .collect();
            schema.into()
        }
    }
};

#[cfg(test)]
#[allow(clippy::inconsistent_digit_grouping)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;
    use serde_with::{DisplayFromStr, de::DeserializeAsWrap, ser::SerializeAsWrap};

    use super::*;

    #[rstest]
    fn timestamp_secs_roundtrip(#[values(0, 1782395622, -1782395622)] secs: i64) {
        let ts = Timestamp::from_secs(secs).unwrap();
        roundtrip_as::<_, TimestampSeconds>(&ts);
        roundtrip_as::<_, TimestampSeconds<DisplayFromStr>>(&ts);
    }

    #[rstest]
    fn timestamp_millis_roundtrip(#[values(0, 1782395622_123, -1782395622_123)] millis: i64) {
        let ts = Timestamp::from_millis(millis).unwrap();
        roundtrip_as::<_, TimestampMilliSeconds>(&ts);
        roundtrip_as::<_, TimestampMilliSeconds<DisplayFromStr>>(&ts);
    }

    #[rstest]
    fn timestamp_micros_roundtrip(
        #[values(0, 1782395622_123456, -1782395622_123456)] micros: i128,
    ) {
        let ts = Timestamp::from_micros(micros).unwrap();
        roundtrip_as::<_, TimestampMicroSeconds>(&ts);
        roundtrip_as::<_, TimestampMicroSeconds<DisplayFromStr>>(&ts);
    }

    #[rstest]
    fn timestamp_nanos_roundtrip(
        #[values(0, 1782395622_123456789, -1782395622_123456789)] nanos: i128,
    ) {
        let ts = Timestamp::from_nanos(nanos).unwrap();
        roundtrip_as::<_, TimestampNanoSeconds>(&ts);
        roundtrip_as::<_, TimestampNanoSeconds<DisplayFromStr>>(&ts);
    }

    // Helper roundtrip
    #[track_caller]
    fn roundtrip_as<T, As>(orig: &T)
    where
        for<'de> As: SerializeAs<T> + DeserializeAs<'de, T>,
        T: PartialEq + Debug,
    {
        let serialized =
            serde_json::to_string(&SerializeAsWrap::<T, As>::new(orig)).expect("JSON: serialize");
        let deserialized: T = serde_json::from_str::<DeserializeAsWrap<T, As>>(&serialized)
            .expect("JSON: deserialize")
            .into_inner();
        assert_eq!(
            &deserialized, orig,
            "deserialized value differs from the original one"
        );
    }
}
