use near_sdk::{
    bs58,
    serde::{Deserialize, Deserializer, Serialize, Serializer, de},
};
use serde_with::{DeserializeAs, SerializeAs};

pub struct Base58;

impl<T> SerializeAs<T> for Base58
where
    T: AsRef<[u8]>,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bs58::encode(source).into_string().serialize(serializer)
    }
}

impl<'de, T> DeserializeAs<'de, T> for Base58
where
    T: TryFrom<Vec<u8>>,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str as Deserialize>::deserialize(deserializer)?;

        let bytes = bs58::decode(s).into_vec().map_err(de::Error::custom)?;

        let length = bytes.len();
        bytes.try_into().map_err(|_| {
            de::Error::custom(format_args!(
                "can't convert a byte vector of length {length} into the output type"
            ))
        })
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
mod abi {
    use super::*;

    use near_sdk::schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject},
    };
    use serde_with::schemars_0_8::JsonSchemaAs;

    impl<T> JsonSchemaAs<T> for Base58 {
        fn schema_name() -> String {
            String::schema_name()
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            // TODO: use #[schemars(extend(...))] when released
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
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
