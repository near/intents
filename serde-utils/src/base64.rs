#![allow(rustdoc::broken_intra_doc_links)]
//! Helper for [`serde_with::base64::Base64`] to implement [`serde_with::schemars_0_8::JsonSchemaAs`] on it.
pub use serde_with::{
    base64::{Alphabet, Standard, UrlSafe},
    formats::{Format, Padded, Unpadded},
};

use derive_more::From;
use near_sdk::serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DeserializeAs, SerializeAs, serde_as};

pub struct Base64<ALPHABET: Alphabet = Standard, PADDING: Format = Padded>(
    ::serde_with::base64::Base64<ALPHABET, PADDING>,
);

impl<T, ALPHABET> SerializeAs<T> for Base64<ALPHABET, Padded>
where
    T: AsRef<[u8]>,
    ALPHABET: Alphabet,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ::serde_with::base64::Base64::<ALPHABET, Padded>::serialize_as(source, serializer)
    }
}

impl<T, ALPHABET> SerializeAs<T> for Base64<ALPHABET, Unpadded>
where
    T: AsRef<[u8]>,
    ALPHABET: Alphabet,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ::serde_with::base64::Base64::<ALPHABET, Unpadded>::serialize_as(source, serializer)
    }
}

impl<'de, T, ALPHABET, FORMAT> DeserializeAs<'de, T> for Base64<ALPHABET, FORMAT>
where
    T: TryFrom<Vec<u8>>,
    ALPHABET: Alphabet,
    FORMAT: Format,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        ::serde_with::base64::Base64::<ALPHABET, FORMAT>::deserialize_as(deserializer)
    }
}

/// Helper type to implement `#[derive(Serialize, Deserialize)]`,
/// as `#[near_bindgen]` doesn't support `#[serde(...)]` attributes on method arguments
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true),
    derive(::near_sdk::schemars::JsonSchema),
    schemars(crate = "::near_sdk::schemars", transparent)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, From)]
#[serde(
    crate = "::near_sdk::serde",
    bound(serialize = "T: AsRef<[u8]>", deserialize = "T: TryFrom<Vec<u8>>")
)]
pub struct AsBase64<T>(#[serde_as(as = "Base64")] pub T);

impl<T> AsBase64<T> {
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
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

    impl<T, ALPHABET, FORMAT> JsonSchemaAs<T> for Base64<ALPHABET, FORMAT>
    where
        ALPHABET: Alphabet,
        FORMAT: Format,
    {
        fn schema_name() -> String {
            String::schema_name()
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            // TODO: use #[schemars(extend(...))] when released
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                extensions: [("contentEncoding", "base64".into())]
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
