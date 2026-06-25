use core::marker::PhantomData;

use serde::{Deserializer, Serializer, de};
use serde_with::{DeserializeAs, Same, SerializeAs};

use crate::Timestamp;

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
                <F as SerializeAs<$int>>::serialize_as(&source.0.$as(), serializer)
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
                ::chrono::DateTime::$from(timestamp).map(Timestamp).ok_or_else(|| de::Error::custom("overflow"))
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
        timestamp,
        from_timestamp_secs,
    }

    pub struct TimestampMilliSeconds: i64 {
        timestamp_millis,
        from_timestamp_millis,
    }

    pub struct TimestampMicroSeconds: i64 {
        timestamp_micros,
        from_timestamp_micros,
    }

    pub struct TimestampNanoSeconds: i64 {
        timestamp_nanos,
        from_timestamp_micros,// TODO
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
                Self::from_nanos(1782415800_123456789).unwrap(),
            ]
            .iter()
            .map(serde_json::to_value)
            .map(Result::unwrap)
            .collect();
            schema.into()
        }
    }
};
