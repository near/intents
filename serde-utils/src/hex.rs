#![allow(rustdoc::broken_intra_doc_links)]
//! Helper for [`serde_with::hex::Hex`] to implement [`serde_with::schemars_0_8::JsonSchemaAs`] on it.
pub use serde_with::formats::{Format, Lowercase, Uppercase};

use derive_more::From;
use near_sdk::serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DeserializeAs, SerializeAs, serde_as};

pub struct Hex<FORMAT: Format = Lowercase>(::serde_with::hex::Hex<FORMAT>);

impl<T> SerializeAs<T> for Hex<Lowercase>
where
    T: AsRef<[u8]>,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ::serde_with::hex::Hex::<Lowercase>::serialize_as(source, serializer)
    }
}

impl<T> SerializeAs<T> for Hex<Uppercase>
where
    T: AsRef<[u8]>,
{
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ::serde_with::hex::Hex::<Uppercase>::serialize_as(source, serializer)
    }
}

impl<'de, T, FORMAT> DeserializeAs<'de, T> for Hex<FORMAT>
where
    T: TryFrom<Vec<u8>>,
    FORMAT: Format,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        ::serde_with::hex::Hex::<FORMAT>::deserialize_as(deserializer)
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
pub struct AsHex<T>(#[serde_as(as = "Hex")] pub T);

impl<T> AsHex<T> {
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
const _: () = {
    use near_sdk::schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject},
    };
    use serde_with::schemars_0_8::JsonSchemaAs;

    impl<T, FORMAT> JsonSchemaAs<T> for Hex<FORMAT>
    where
        FORMAT: Format,
    {
        fn schema_name() -> String {
            String::schema_name()
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                ..Default::default()
            }
            .into()
        }

        fn is_referenceable() -> bool {
            false
        }
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::serde_json;

    #[test]
    fn serialize_vec() {
        let val = AsHex(vec![0xde, 0xad, 0xbe, 0xef]);
        let json = serde_json::to_string(&val).unwrap();
        assert_eq!(json, r#""deadbeef""#);
    }

    #[test]
    fn deserialize_vec() {
        let val: AsHex<Vec<u8>> = serde_json::from_str(r#""deadbeef""#).unwrap();
        assert_eq!(val.0, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn roundtrip_vec() {
        let original = vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
        let wrapped = AsHex(original.clone());
        let json = serde_json::to_string(&wrapped).unwrap();
        let recovered: AsHex<Vec<u8>> = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.0, original);
    }

    #[test]
    fn serialize_fixed_array() {
        let val = AsHex([0xff, 0x00, 0xab]);
        let json = serde_json::to_string(&val).unwrap();
        assert_eq!(json, r#""ff00ab""#);
    }

    #[test]
    fn deserialize_fixed_array() {
        let val: AsHex<[u8; 3]> = serde_json::from_str(r#""ff00ab""#).unwrap();
        assert_eq!(val.0, [0xff, 0x00, 0xab]);
    }

    #[test]
    fn empty_bytes() {
        let val = AsHex(Vec::<u8>::new());
        let json = serde_json::to_string(&val).unwrap();
        assert_eq!(json, r#""""#);

        let recovered: AsHex<Vec<u8>> = serde_json::from_str(&json).unwrap();
        assert!(recovered.0.is_empty());
    }

    #[test]
    fn into_inner() {
        let data = vec![1, 2, 3];
        let wrapped = AsHex(data.clone());
        assert_eq!(wrapped.into_inner(), data);
    }

    #[test]
    fn from_impl() {
        let data = vec![1, 2, 3];
        let wrapped: AsHex<Vec<u8>> = data.clone().into();
        assert_eq!(wrapped.0, data);
    }

    #[test]
    fn deserialize_uppercase_input() {
        let val: AsHex<Vec<u8>> = serde_json::from_str(r#""DEADBEEF""#).unwrap();
        assert_eq!(val.0, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn deserialize_mixed_case_input() {
        let val: AsHex<Vec<u8>> = serde_json::from_str(r#""DeAdBeEf""#).unwrap();
        assert_eq!(val.0, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn deserialize_invalid_hex() {
        let result = serde_json::from_str::<AsHex<Vec<u8>>>(r#""xyz""#);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_odd_length() {
        let result = serde_json::from_str::<AsHex<Vec<u8>>>(r#""abc""#);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_wrong_length_for_fixed_array() {
        let result = serde_json::from_str::<AsHex<[u8; 4]>>(r#""deadbeefaa""#);
        assert!(result.is_err());
    }
}
