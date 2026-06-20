use core::str;
use std::borrow::Cow;
use std::fmt::Debug;

use tlb_ton::{MsgAddress, StringError};

#[cfg(feature = "binary")]
mod binary;
#[cfg(feature = "cell")]
mod cell;
#[cfg(feature = "text")]
mod text;

pub struct TonConnectPayloadContext<'a> {
    pub address: MsgAddress,
    pub domain: Cow<'a, str>,
    pub timestamp: u64,
}

impl TonConnectPayloadContext<'_> {
    // See https://docs.tonconsole.com/academy/sign-data#how-the-signature-is-built
    #[cfg(any(feature = "binary", feature = "text"))]
    pub fn create_payload_hash(
        &self,
        payload_prefix: &[u8],
        payload: &[u8],
    ) -> Result<defuse_crypto::CryptoHash, StringError> {
        use defuse_digest::{Digest, sha2::Sha256};

        let domain_len = u32::try_from(self.domain.len())
            .map_err(|_| tlb_ton::Error::custom("domain: overflow"))?;
        let payload_len = u32::try_from(payload.len())
            .map_err(|_| tlb_ton::Error::custom("payload: overflow"))?;

        Ok(Sha256::new_with_prefix(b"\xFF\xFFton-connect/sign-data/")
            .chain_update(self.address.workchain_id.to_be_bytes())
            .chain_update(self.address.address)
            .chain_update(domain_len.to_be_bytes())
            .chain_update(self.domain.as_bytes())
            .chain_update(self.timestamp.to_be_bytes())
            .chain_update(payload_prefix)
            .chain_update(payload_len.to_be_bytes())
            .chain_update(payload)
            .finalize()
            .into())
    }
}

pub trait PayloadSchema {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<defuse_crypto::CryptoHash, StringError>;
}

/// See <https://docs.tonconsole.com/academy/sign-data#choosing-the-right-format>
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema)),
    serde(tag = "type", rename_all = "snake_case")
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TonConnectPayloadSchema {
    #[cfg(feature = "text")]
    Text(text::TextPayload),
    #[cfg(feature = "binary")]
    Binary(binary::BinaryPayload),
    #[cfg(feature = "cell")]
    Cell(cell::CellPayload),
}

impl TonConnectPayloadSchema {
    #[cfg(feature = "text")]
    pub fn text(txt: impl Into<String>) -> Self {
        Self::Text(text::TextPayload { text: txt.into() })
    }

    #[cfg(feature = "binary")]
    pub fn binary(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Binary(binary::BinaryPayload {
            bytes: bytes.into(),
        })
    }

    #[cfg(feature = "cell")]
    pub const fn cell(schema_crc: u32, cell: tlb_ton::Cell) -> Self {
        Self::Cell(cell::CellPayload { schema_crc, cell })
    }
}

impl PayloadSchema for TonConnectPayloadSchema {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<defuse_crypto::CryptoHash, StringError> {
        match self {
            #[cfg(feature = "text")]
            Self::Text(payload) => payload.hash_with_context(context),
            #[cfg(feature = "binary")]
            Self::Binary(payload) => payload.hash_with_context(context),
            #[cfg(feature = "cell")]
            Self::Cell(payload) => payload.hash_with_context(context),
        }
    }
}
