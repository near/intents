//! This module presents traits according to [multi-token metadata extension](https://github.com/near/NEPs/blob/master/specs/Standards/Tokens/MultiToken/Metadata.md)

use crate::TokenId;
use crate::enumeration::MultiTokenEnumeration;
use crate::metadata::adapters::As;
use borsh::schema::{Declaration, Definition};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use chrono::{DateTime, Utc};
use defuse_borsh_utils::adapters;
use near_sdk::near;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::Schema;
use serde_with::serde_as;
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

pub type MetadataId = String;

#[derive(Debug, Clone)]
#[near(serializers = [json, borsh])]
pub struct MTContractMetadata {
    pub spec: String, // "a string that MUST be formatted mt-1.0.0" or whatever the spec version is used.
    pub name: String,
}

#[derive(Debug, Clone)]
#[skip_serializing_none]
#[near(serializers = [json, borsh])]
pub struct MTBaseTokenMetadata {
    /// Human‐readable name of the base (e.g., "Silver Swords" or "Metaverse 3")
    pub name: String,

    /// Unique identifier for this metadata entry
    pub id: MetadataId,

    /// Abbreviated symbol for the token (e.g., "MOCHI"), or `None` if unset
    pub symbol: Option<String>,

    /// Data URL for a small icon image, or `None`
    pub icon: Option<String>,

    /// Number of decimals (useful if this base represents an FT‐style token), or `None`
    pub decimals: Option<u8>,

    /// Centralized gateway URL for reliably accessing decentralized storage assets referenced by `reference` or `media`, or `None`
    pub base_uri: Option<String>,

    /// URL pointing to a JSON file with additional info, or `None`
    pub reference: Option<String>,

    /// Number of copies of this set of metadata that existed when the token was minted, or `None`
    pub copies: Option<u64>,

    /// Base64‐encoded SHA-256 hash of the JSON from `reference`; required if `reference` is set, or `None`
    pub reference_hash: Option<String>,
}

#[derive(Debug, Clone)]
#[skip_serializing_none]
#[near(serializers = [json, borsh])]
pub struct MTTokenMetadata {
    /// Title of this specific token (e.g., "Arch Nemesis: Mail Carrier" or "Parcel #5055"), or `None`
    pub title: Option<String>,

    /// Free-form description of this token, or `None`
    pub description: Option<String>,

    /// URL to associated media (ideally decentralized, content-addressed storage), or `None`
    pub media: Option<String>,

    /// Base64‐encoded SHA-256 hash of the content referenced by `media`; required if `media` is set, or `None`
    pub media_hash: Option<String>,

    /// Unix epoch in milliseconds or RFC3339 when this token was issued or minted, or `None`
    pub issued_at: Option<DatetimeUtcWrapper>,

    /// Unix epoch in milliseconds or RFC3339 when this token expires, or `None`
    pub expires_at: Option<DatetimeUtcWrapper>,

    /// Unix epoch in milliseconds or RFC3339 when this token starts being valid, or `None`
    pub starts_at: Option<DatetimeUtcWrapper>,

    /// Unix epoch in milliseconds or RFC3339 when this token metadata was last updated, or `None`
    pub updated_at: Option<DatetimeUtcWrapper>,

    /// Anything extra the MT wants to store on-chain (can be stringified JSON), or `None`
    pub extra: Option<String>,

    /// URL to an off-chain JSON file with more info, or `None`
    pub reference: Option<String>,

    /// Base64‐encoded SHA-256 hash of the JSON from `reference`; required if `reference` is set, or `None`
    pub reference_hash: Option<String>,
}

#[derive(Debug, Clone)]
#[near(serializers = [json, borsh])]
pub struct MTTokenMetadataAll {
    pub base: MTBaseTokenMetadata,
    pub token: MTTokenMetadata,
}

pub trait MultiTokenMetadata {
    /// Returns the contract‐level metadata (spec + name).
    fn mt_metadata_contract(&self) -> MTContractMetadata;

    /// For a list of `token_ids`, returns a vector of combined `(base, token)` metadata.
    fn mt_metadata_token_all(&self, token_ids: Vec<TokenId>) -> Vec<Option<MTTokenMetadataAll>>;

    /// Given `token_ids`, returns each token’s `MTTokenMetadata` or `None` if absent.
    fn mt_metadata_token_by_token_id(
        &self,
        token_ids: Vec<TokenId>,
    ) -> Vec<Option<MTTokenMetadata>>;

    /// Given `token_ids`, returns each token’s `MTBaseTokenMetadata` or `None` if absent.
    fn mt_metadata_base_by_token_id(
        &self,
        token_ids: Vec<TokenId>,
    ) -> Vec<Option<MTBaseTokenMetadata>>;

    /// Given a list of `base_metadata_ids`, returns each `MTBaseTokenMetadata` or `None` if absent.
    fn mt_metadata_base_by_metadata_id(
        &self,
        base_metadata_ids: Vec<MetadataId>,
    ) -> Vec<Option<MTBaseTokenMetadata>>;
}

/// The contract must implement the following view method if using [multi-token enumeration standard](https://nomicon.io/Standards/Tokens/MultiToken/Enumeration#interface).
pub trait MultiTokenMetadataEnumeration: MultiTokenMetadata + MultiTokenEnumeration {
    /// Get list of all base metadata for the contract, with pagination.
    ///
    /// # Arguments
    /// * `from_index`: an optional string representing an unsigned 128-bit integer,
    ///    indicating the starting index
    /// * `limit`: an optional u64 indicating the maximum number of entries to return
    ///
    /// # Returns
    /// A vector of `MTBaseTokenMetadata` objects, or an empty vector if none.
    fn mt_tokens_base_metadata_all(
        &self,
        from_index: Option<String>,
        limit: Option<u64>,
    ) -> Vec<MTBaseTokenMetadata>;
}

/// A wrapper that implements Borsh de-/serialization for `Datetime<Utc>`
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "::near_sdk::serde")]
#[serde_as]
pub struct DatetimeUtcWrapper(
    #[serde_as(as = "PickFirst<(_, serde_with::TimestampMilliSeconds)>")]
    #[borsh(
        deserialize_with = "As::<adapters::TimestampMilliSeconds>::deserialize",
        serialize_with = "As::<adapters::TimestampMilliSeconds>::serialize"
    )]
    pub DateTime<Utc>,
);

impl JsonSchema for DatetimeUtcWrapper {
    fn schema_name() -> String {
        "DatetimeUtcWrapper".to_owned()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<u64>()
    }
}

impl BorshSchema for DatetimeUtcWrapper {
    fn add_definitions_recursively(definitions: &mut BTreeMap<Declaration, Definition>) {
        <u64 as BorshSchema>::add_definitions_recursively(definitions);
    }

    fn declaration() -> Declaration {
        <u64 as BorshSchema>::declaration()
    }
}

#[cfg(test)]
mod tests {
    use crate::metadata::DatetimeUtcWrapper;
    use chrono::DateTime;
    use hex::FromHex;
    use near_sdk::borsh;

    #[test]
    fn test_datetime_utc_wrapper_borsh() {
        let timestamp = DateTime::from_timestamp(1747772412, 0).unwrap();
        let wrapped = DatetimeUtcWrapper(timestamp);
        let encoded = borsh::to_vec(&wrapped).unwrap();
        assert_eq!(encoded, Vec::from_hex("60905aef96010000").unwrap());
        let actual_wrapped: DatetimeUtcWrapper = borsh::from_slice(encoded.as_slice()).unwrap();
        assert_eq!(actual_wrapped.0, wrapped.0);
    }
}
