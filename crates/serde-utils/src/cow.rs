use std::{borrow::Cow, marker::PhantomData};

use serde::{Deserializer, Serializer, ser::SerializeSeq};
use serde_with::{DeserializeAs, SerializeAs, ser::SerializeAsWrap};

pub struct AsCow<As: ?Sized>(PhantomData<As>);

impl<T, As> SerializeAs<Cow<'_, [T]>> for AsCow<As>
where
    T: Clone,
    As: SerializeAs<T>,
{
    fn serialize_as<S>(source: &Cow<'_, [T]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(source.len()))?;

        for item in source.iter() {
            seq.serialize_element(&SerializeAsWrap::<T, As>::new(item))?;
        }

        seq.end()
    }
}

impl<'de, T, As> DeserializeAs<'de, Cow<'static, [T]>> for AsCow<As>
where
    T: Clone,
    As: DeserializeAs<'de, T>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<Cow<'static, [T]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        <Vec<As> as DeserializeAs<'de, Vec<T>>>::deserialize_as(deserializer).map(Cow::Owned)
    }
}

#[cfg(feature = "schemars-v0_8")]
const _: () = {
    use schemars::{SchemaGenerator, schema::Schema};
    use serde_with::schemars_0_8::JsonSchemaAs;

    impl<T, As> JsonSchemaAs<Cow<'_, [T]>> for AsCow<As>
    where
        T: Clone,
        As: JsonSchemaAs<T>,
    {
        fn schema_name() -> String {
            <Vec<As> as JsonSchemaAs<Vec<T>>>::schema_name()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            <Vec<As> as JsonSchemaAs<Vec<T>>>::json_schema(generator)
        }

        fn is_referenceable() -> bool {
            <Vec<As> as JsonSchemaAs<Vec<T>>>::is_referenceable()
        }
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_with::{DisplayFromStr, serde_as};

    #[serde_as]
    #[derive(Debug, Serialize, Deserialize)]
    struct S<'a> {
        #[serde_as(as = "AsCow<DisplayFromStr>")]
        amounts: Cow<'a, [u128]>,
    }

    #[test]
    fn serializes_as_strings() {
        let s = S {
            amounts: Cow::Owned(vec![1, u128::MAX]),
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, format!(r#"{{"amounts":["1","{}"]}}"#, u128::MAX));
    }

    #[test]
    fn roundtrip() {
        let s = S {
            amounts: Cow::Owned(vec![0, 42, u128::MAX]),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: S = serde_json::from_str(&json).unwrap();
        assert_eq!(back.amounts.into_owned(), vec![0, 42, u128::MAX]);
    }
}
