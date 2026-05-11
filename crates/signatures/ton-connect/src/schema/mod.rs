use core::str;
use std::borrow::Cow;
use std::fmt::Debug;

use sha2::digest;
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
    pub fn create_payload_hash<D: digest::Digest<OutputSize = digest::consts::U32>>(
        &self,
        payload_prefix: &[u8],
        payload: &[u8],
    ) -> Result<defuse_crypto::CryptoHash, StringError> {
        let domain_len = u32::try_from(self.domain.len())
            .map_err(|_| tlb_ton::Error::custom("domain: overflow"))?;
        let payload_len = u32::try_from(payload.len())
            .map_err(|_| tlb_ton::Error::custom("payload: overflow"))?;

        let bytes = [
            [0xff, 0xff].as_slice(),
            b"ton-connect/sign-data/",
            &self.address.workchain_id.to_be_bytes(),
            self.address.address.as_ref(),
            &domain_len.to_be_bytes(),
            self.domain.as_bytes(),
            &self.timestamp.to_be_bytes(),
            payload_prefix,
            &payload_len.to_be_bytes(),
            payload,
        ]
        .concat();

        Ok(Into::<[u8; 32]>::into(D::digest(&bytes)))
    }
}

pub trait PayloadSchema {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<defuse_crypto::CryptoHash, StringError>;
}

/// See <https://docs.tonconsole.com/academy/sign-data#choosing-the-right-format>
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(tag = "type", rename_all = "snake_case"),
    serde(bound = ""),
    cfg_attr(feature = "abi", derive(::schemars::JsonSchema))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TonConnectPayloadSchema<D> {
    #[cfg(feature = "text")]
    Text(text::TextPayload<D>),
    #[cfg(feature = "binary")]
    Binary(binary::BinaryPayload<D>),
    #[cfg(feature = "cell")]
    Cell(cell::CellPayload<D>),
}

impl<D: digest::Digest<OutputSize = digest::consts::U32>> TonConnectPayloadSchema<D> {
    #[cfg(feature = "text")]
    pub fn text(txt: impl Into<String>) -> Self {
        Self::Text(text::TextPayload {
            text: txt.into(),
            _phantom: std::marker::PhantomData::<D>,
        })
    }

    #[cfg(feature = "binary")]
    pub fn binary(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Binary(binary::BinaryPayload {
            bytes: bytes.into(),
            _phantom: std::marker::PhantomData::<D>,
        })
    }

    #[cfg(feature = "cell")]
    pub const fn cell(schema_crc: u32, cell: tlb_ton::Cell) -> Self {
        Self::Cell(cell::CellPayload {
            schema_crc,
            cell,
            _phantom: std::marker::PhantomData::<D>,
        })
    }
}

impl<D: digest::Digest<OutputSize = digest::consts::U32>> PayloadSchema
    for TonConnectPayloadSchema<D>
{
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
            #[cfg(not(any(feature = "text", feature = "binary", feature = "cell")))]
            _ => unreachable!(),
        }
    }
}
