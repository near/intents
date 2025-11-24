use near_sdk::{
    bs58,
    serde::{Deserialize, Deserializer, Serialize, Serializer, de},
};
use serde_with::{DeserializeAs, SerializeAs};

#[cfg(all(feature = "abi", not(target_arch = "wasm32")))]
use near_sdk::NearSchema;

#[cfg_attr(all(feature = "abi", not(target_arch = "wasm32")), derive(NearSchema))]
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
