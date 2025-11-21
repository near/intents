use core::str;
use std::borrow::Cow;
use std::fmt::Debug;

use defuse_serde_utils::{base64::Base64, tlb::AsBoC};
use impl_tools::autoimpl;
use near_sdk::{env, near};
use serde_with::serde_as;
use tlb_ton::{
    Cell, Error, MsgAddress, StringError,
    r#as::{Ref, SnakeData},
    bits::ser::BitWriterExt,
    ser::{CellBuilder, CellBuilderError, CellSerialize, CellSerializeExt},
};

pub struct TonConnectPayloadContext {
    pub address: MsgAddress,
    pub domain: String,
    pub timestamp: u64,
}

impl TonConnectPayloadContext {
    // See https://docs.tonconsole.com/academy/sign-data#how-the-signature-is-built
    pub fn create_payload_hash(
        &self,
        payload_prefix: &[u8],
        payload: &[u8],
    ) -> Result<near_sdk::CryptoHash, StringError> {
        let domain_len =
            u32::try_from(self.domain.len()).map_err(|_| Error::custom("domain: overflow"))?;
        let payload_len =
            u32::try_from(payload.len()).map_err(|_| Error::custom("payload: overflow"))?;

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

        Ok(env::sha256_array(&bytes))
    }
}

pub trait PayloadSchema: Debug + Clone + PartialEq + Eq {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<near_sdk::CryptoHash, StringError>;
}

/// See <https://docs.tonconsole.com/academy/sign-data#choosing-the-right-format>
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TonConnectPayloadSchema {
    #[cfg(feature = "text")]
    Text(TextPayload),
    #[cfg(feature = "binary")]
    Binary(BinaryPayload),
    #[cfg(feature = "cell")]
    Cell(CellPayload),
}

impl TonConnectPayloadSchema {
    #[cfg(feature = "text")]
    pub fn text(txt: &impl ToString) -> Self {
        Self::Text(TextPayload {
            text: txt.to_string(),
        })
    }

    #[cfg(feature = "binary")]
    pub fn binary(bytes: &[u8]) -> Self {
        Self::Binary(BinaryPayload {
            bytes: bytes.to_vec(),
        })
    }

    #[cfg(feature = "cell")]
    pub const fn cell(schema_crc: u32, cell: Cell) -> Self {
        Self::Cell(CellPayload { schema_crc, cell })
    }

    #[cfg(feature = "text")]
    pub fn try_extract_text(&self) -> Option<String> {
        if let Self::Text(payload) = self {
            Some(payload.text.clone())
        } else {
            None
        }
    }
}

impl PayloadSchema for TonConnectPayloadSchema {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<near_sdk::CryptoHash, StringError> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[autoimpl(Deref using self.text)]
pub struct TextPayload {
    text: String,
}

impl PayloadSchema for TextPayload {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<near_sdk::CryptoHash, StringError> {
        context.create_payload_hash(b"txt", self.as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[autoimpl(Deref using self.bytes)]
pub struct BinaryPayload {
    #[serde_as(as = "Base64")]
    bytes: Vec<u8>,
}

impl PayloadSchema for BinaryPayload {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<near_sdk::CryptoHash, StringError> {
        context.create_payload_hash(b"bin", self.as_slice())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
pub struct CellPayload {
    schema_crc: u32,
    #[serde_as(as = "AsBoC<Base64>")]
    cell: Cell,
}

/// ```tlb
/// message#75569022 schema_hash:uint32 timestamp:uint64 userAddress:MsgAddress
///                  {n:#} appDomain:^(SnakeData ~n) payload:^Cell = Message;
/// ```
#[derive(Debug, Clone)]
pub struct TonConnectCellMessage<'a, T = Cell> {
    pub schema_crc: u32,
    pub timestamp: u64,
    pub user_address: Cow<'a, MsgAddress>,
    pub app_domain: Cow<'a, str>,
    pub payload: T,
}

/// ```tlb
/// message#75569022
/// ```
#[allow(clippy::unreadable_literal)]
const MESSAGE_TAG: u32 = 0x75569022;

impl<T> CellSerialize for TonConnectCellMessage<'_, T>
where
    T: CellSerialize,
{
    fn store(&self, builder: &mut CellBuilder) -> Result<(), CellBuilderError> {
        builder
            // message#75569022
            .pack(MESSAGE_TAG)?
            // schema_hash:uint32
            .pack(self.schema_crc)?
            // timestamp:uint64
            .pack(self.timestamp)?
            // userAddress:MsgAddress
            .pack(&self.user_address)?
            // {n:#} appDomain:^(SnakeData ~n)
            .store_as::<_, Ref<SnakeData>>(self.app_domain.as_ref())?
            // payload:^Cell
            .store_as::<_, Ref>(&self.payload)?;
        Ok(())
    }
}

impl PayloadSchema for CellPayload {
    fn hash_with_context(
        &self,
        context: TonConnectPayloadContext,
    ) -> Result<near_sdk::CryptoHash, StringError> {
        let cell = TonConnectCellMessage {
            schema_crc: self.schema_crc,
            timestamp: context.timestamp,
            user_address: Cow::Borrowed(&context.address),
            app_domain: Cow::Borrowed(context.domain.as_str()),
            payload: self.cell.clone(),
        }
        .to_cell()?;

        // use host function for recursive hash calculation
        Ok(cell.hash_digest::<defuse_near_utils::digest::Sha256>())
    }
}
