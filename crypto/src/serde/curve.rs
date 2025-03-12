use core::marker::PhantomData;

use near_sdk::serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_with::{DeserializeAs, SerializeAs};

use crate::TypedCurve;

pub struct AsCurve<C: TypedCurve>(PhantomData<C>);

impl<C: TypedCurve, const N: usize> SerializeAs<[u8; N]> for AsCurve<C> {
    fn serialize_as<S>(source: &[u8; N], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        C::to_base58(source).serialize(serializer)
    }
}

impl<'de, C: TypedCurve, const N: usize> DeserializeAs<'de, [u8; N]> for AsCurve<C> {
    fn deserialize_as<D>(deserializer: D) -> Result<[u8; N], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str as Deserialize>::deserialize(deserializer)?;
        C::parse_base58(s).map_err(de::Error::custom)
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod abi {
    use super::*;

    use near_sdk::schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject, StringValidation},
    };
    use serde_with::schemars_0_8::JsonSchemaAs;

    impl<C: TypedCurve, const N: usize> JsonSchemaAs<[u8; N]> for AsCurve<C> {
        fn schema_name() -> String {
            String::schema_name()
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                string: Some(
                    StringValidation {
                        pattern: Some(format!("^{}:", C::CURVE_TYPE)),
                        ..Default::default()
                    }
                    .into(),
                ),
                extensions: [("contentEncoding", "base58".into())]
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
                ..Default::default()
            }
            .into()
        }

        fn is_referenceable() -> bool {
            false
        }
    }
}
