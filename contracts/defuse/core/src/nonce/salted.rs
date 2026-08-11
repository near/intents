use borsh::{BorshDeserialize, BorshSerialize};
use hex::FromHex;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::{
    fmt::{self, Debug},
    str::FromStr,
};

use crate::Result;

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "borsh-schema", derive(::borsh::BorshSchema))]
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    SerializeDisplay,
    DeserializeFromStr,
    BorshSerialize,
    BorshDeserialize,
)]
#[repr(transparent)]
pub struct Salt(pub [u8; 4]);

impl fmt::Debug for Salt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl fmt::Display for Salt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl FromStr for Salt {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        FromHex::from_hex(s).map(Self)
    }
}

#[cfg(feature = "schemars-v0_8")]
const _: () = {
    use schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject},
    };

    impl JsonSchema for Salt {
        fn schema_name() -> String {
            String::schema_name()
        }

        fn is_referenceable() -> bool {
            false
        }

        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                extensions: std::iter::once(("contentEncoding", "hex".into()))
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
                ..Default::default()
            }
            .into()
        }
    }
};

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct SaltedNonce<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub salt: Salt,
    pub nonce: T,
}

impl<T> SaltedNonce<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub const fn new(salt: Salt, nonce: T) -> Self {
        Self { salt, nonce }
    }
}
