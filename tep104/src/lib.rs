//! [TEP-104](https://github.com/ton-blockchain/TEPs/pull/104): Data Signatures

use defuse_crypto::{Curve, Ed25519, Payload, SignedPayload, serde::AsCurve};
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::serde_as;
use tlb::{
    r#as::Text,
    ser::{CellBuilder, CellBuilderError, CellSerialize, CellSerializeExt},
};

#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct Tep104Payload {
    #[serde(flatten)]
    pub payload: Tep104SchemaPayload,
    pub timestamp: u64, // TODO: chrono?
}

impl Tep104Payload {
    // TODO: rename
    // uint32be(schema_crc) ++ uint64be(timestamp) ++ cell_hash(X)
    fn data(&self) -> Vec<u8> {
        [
            // uint32be(schema_crc)
            self.schema_crc().to_be_bytes().as_slice(),
            // uint64be(timestamp)
            self.timestamp.to_be_bytes().as_slice(),
            // cell_hash(X)
            env::sha256(
                &self
                    .payload
                    .to_cell()
                    // TODO
                    .unwrap()
                    .repr(),
            )
            .as_slice(),
        ]
        .concat()
    }
}

#[near(serializers = [json])]
#[serde(tag = "schema", rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub enum Tep104SchemaPayload {
    /// This schema is used to sign UTF-8 text messages using
    /// _snake format_ (per [TEP-64](https://github.com/ton-blockchain/TEPs/blob/master/text/0064-token-data-standard.md)).
    /// ```
    /// crc32('plaintext text:Text = PayloadCell') = 0x754bf91b
    /// ```
    Plaintext {
        // TODO: check timestamp
        payload: String,
    },
    // TODO: app_data
    // TODO: custom?
}

impl Tep104SchemaPayload {
    pub fn schema_crc(&self) -> u32 {
        match self {
            // crc32('plaintext text:Text = PayloadCell') = 0x754bf91b
            Self::Plaintext { .. } => 0x754bf91b,
        }
    }
}

impl CellSerialize for Tep104SchemaPayload {
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        match self {
            Self::Plaintext { payload } => {
                // plaintext text:Text = PayloadCell
                builder.store_as::<_, Text>(payload)?;
            }
        }
        Ok(())
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[autoimpl(Deref using self.payload)]
#[derive(Debug, Clone)]
pub struct SignedTep104Payload {
    #[serde(flatten)]
    pub payload: Tep104Payload,

    #[serde_as(as = "AsCurve<Ed25519>")]
    pub public_key: <Ed25519 as Curve>::PublicKey,
    #[serde_as(as = "AsCurve<Ed25519>")]
    pub signature: <Ed25519 as Curve>::Signature,
}

impl Payload for SignedTep104Payload {
    #[inline]
    fn hash(&self) -> near_sdk::CryptoHash {
        env::sha256_array(&self.payload.data())
    }
}

impl SignedPayload for SignedTep104Payload {
    type PublicKey = <Ed25519 as Curve>::PublicKey;

    #[inline]
    fn verify(&self) -> Option<Self::PublicKey> {
        Ed25519::verify(&self.signature, &self.payload.data(), &self.public_key)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn verify() {
        let signed = SignedTep104Payload {
            payload: Tep104Payload {
                payload: Tep104SchemaPayload::Plaintext {
                    payload: "Hello, TON!".to_string(),
                },
                timestamp: 1747675783,
            },
            public_key: todo!(),
            signature: todo!(),
        };

        assert_eq!(signed.verify(), Some(signed.public_key));
    }
}
