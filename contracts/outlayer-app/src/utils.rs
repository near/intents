use near_sdk::{
    borsh,
    borsh::{BorshDeserialize, BorshSerialize},
};

#[derive(Debug, Clone, PartialEq, Eq, near_sdk::serde::Serialize, near_sdk::serde::Deserialize)]
#[serde(crate = "near_sdk::serde", transparent)]
pub struct Url(pub url::Url);

impl Url {
    pub fn parse(s: &str) -> Result<Self, url::ParseError> {
        url::Url::parse(s).map(Url)
    }
}

impl From<url::Url> for Url {
    fn from(u: url::Url) -> Self {
        Self(u)
    }
}

impl TryFrom<String> for Url {
    type Error = url::ParseError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        url::Url::parse(&s).map(Self)
    }
}

impl BorshSerialize for Url {
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        BorshSerialize::serialize(self.0.as_str(), writer)
    }
}

impl BorshDeserialize for Url {
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let s = String::deserialize_reader(reader)?;
        url::Url::parse(&s)
            .map(Url)
            .map_err(|e| borsh::io::Error::new(borsh::io::ErrorKind::InvalidData, e.to_string()))
    }
}

#[cfg(feature = "abi")]
const _: () = {
    use near_sdk::schemars::{JsonSchema, r#gen::SchemaGenerator, schema::Schema};
    impl JsonSchema for Url {
        fn schema_name() -> String {
            String::schema_name()
        }
        fn is_referenceable() -> bool {
            false
        }
        fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
            <String as JsonSchema>::json_schema(_gen)
        }
    }
};

#[cfg(feature = "abi")]
impl borsh::BorshSchema for Url {
    fn declaration() -> borsh::schema::Declaration {
        <String as borsh::BorshSchema>::declaration()
    }
    fn add_definitions_recursively(
        definitions: &mut std::collections::BTreeMap<
            borsh::schema::Declaration,
            borsh::schema::Definition,
        >,
    ) {
        <String as borsh::BorshSchema>::add_definitions_recursively(definitions);
    }
}
