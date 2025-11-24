use core::str;
use std::borrow::Cow;
use std::fmt::Debug;

use defuse_serde_utils::{base64::Base64, tlb::AsBoC};
use near_sdk::near;
use serde_with::serde_as;
use tlb_ton::{
    Cell, MsgAddress, StringError,
    r#as::{Ref, SnakeData},
    bits::ser::BitWriterExt,
    ser::{CellBuilder, CellBuilderError, CellSerialize, CellSerializeExt},
};

use crate::schema::{PayloadSchema, TonConnectPayloadContext};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(arbitrary::Arbitrary))]
#[near(serializers = [json])]
pub struct CellPayload {
    pub schema_crc: u32,
    #[serde_as(as = "AsBoC<Base64>")]
    pub cell: Cell,
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
            app_domain: context.domain,
            payload: self.cell.clone(),
        }
        .to_cell()?;

        // use host function for recursive hash calculation
        Ok(cell.hash_digest::<defuse_near_utils::digest::Sha256>())
    }
}
