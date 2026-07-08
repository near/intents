use defuse_digest::sha2::Sha256;
use tlb_ton::{
    Cell, MsgAddress, Ref, SnakeData,
    bits::{NoArgs, ser::BitWriterExt},
    ser::{CellBuilder, CellBuilderError, CellSerialize, CellSerializeExt},
};

/// ```tlb
/// message#75569022 schema_hash:uint32 timestamp:uint64 userAddress:MsgAddress
///                  {n:#} appDomain:^(SnakeData ~n) payload:^Cell = Message;
/// ```
#[derive(Debug, Clone)]
pub struct TonConnectCellMessage<'a, T = Cell> {
    pub schema_crc: u32,
    pub timestamp: u64,
    pub user_address: &'a MsgAddress,
    pub app_domain: &'a str,
    pub payload: T,
}

impl<T> TonConnectCellMessage<'_, T>
where
    Self: CellSerialize<Args: NoArgs>,
{
    #[inline]
    pub fn hash(&self) -> Option<[u8; 32]> {
        let cell = self.to_cell(NoArgs::EMPTY).ok()?;
        Some(cell.hash_digest::<Sha256>())
    }
}

/// ```tlb
/// message#75569022
/// ```
#[allow(clippy::unreadable_literal)]
const MESSAGE_TAG: u32 = 0x75569022;

impl<T> CellSerialize for TonConnectCellMessage<'_, T>
where
    T: CellSerialize<Args: NoArgs>,
{
    type Args = ();

    fn store(&self, builder: &mut CellBuilder, (): Self::Args) -> Result<(), CellBuilderError> {
        builder
            // message#75569022
            .pack(MESSAGE_TAG, ())?
            // schema_hash:uint32
            .pack(self.schema_crc, ())?
            // timestamp:uint64
            .pack(self.timestamp, ())?
            // userAddress:MsgAddress
            .pack(self.user_address, ())?
            // {n:#} appDomain:^(SnakeData ~n)
            .store_as::<_, Ref<SnakeData>>(self.app_domain, ())?
            // payload:^Cell
            .store_as::<_, Ref>(&self.payload, NoArgs::EMPTY)?;
        Ok(())
    }
}
