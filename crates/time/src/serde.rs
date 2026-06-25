use core::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_with::{DeserializeAs, Same, SerializeAs};

use crate::Timestamp;

/// An adaptor to de/serialize [`jiff::Timestamp`] according to RFC-3339.
///
/// [`jiff::Timestamp`] already implements `serde` traits according to
/// RFC-3339, but, unfortunately, it doesn't implement [`schemars::JsonSchema`].
/// This helper implements them all.
pub struct Rfc3339;

impl SerializeAs<Timestamp> for Rfc3339 {
    fn serialize_as<S>(source: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // `jiff::Timestamp` already implements serialization according
        // to RFC-3339
        source.serialize(serializer)
    }
}

impl<'de> DeserializeAs<'de, Timestamp> for Rfc3339 {
    fn deserialize_as<D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        // `jiff::Timestamp` already implements deserialization according
        // to RFC-3339
        Timestamp::deserialize(deserializer)
    }
}

#[cfg(feature = "schemars_v0_8")]
const _: () = {
    use schemars::{
        JsonSchema, SchemaGenerator,
        schema::{Metadata, Schema},
    };
    use serde_with::schemars_0_8::JsonSchemaAs;

    impl JsonSchemaAs<Timestamp> for Rfc3339 {
        #[inline]
        fn is_referenceable() -> bool {
            true
        }

        #[inline]
        fn schema_name() -> String {
            stringify!(Rfc3339).into()
        }

        #[inline]
        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            let mut schema = <String as JsonSchema>::json_schema(generator).into_object();

            schema.metadata = Some(
                Metadata {
                    examples: vec![
                        Timestamp::UNIX_EPOCH.to_string().into(),
                        Timestamp::new(1782305944, 123456789)
                            .unwrap()
                            .display_with_offset(Offset::constant(4))
                            .to_string()
                            .into(),
                    ],
                    ..Default::default()
                }
                .into(),
            );

            schema.into()
        }
    }
};

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
                <F as SerializeAs<$int>>::serialize_as(&source.$as(), serializer)
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
                Timestamp::$from(timestamp).map_err(de::Error::custom)
            }
        }

        #[cfg(feature = "schemars_v0_8")]
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
