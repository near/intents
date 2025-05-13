use defuse_core::{
    events::DefuseEvent,
    intents::tokens::{FtWithdraw, MtWithdraw, NftWithdraw},
};

/// A withdraw is done through a unified function using the MtToken interface.
/// This struct helps in transferring the withdraw event information from the
/// type-specific withdraw (Ft/Mt/Nft, etc), to the unified part that performs
/// the withdrawal.
#[derive(Debug, Clone)]
pub enum WithdrawEventMediator {
    FtWithdraw(FtWithdraw),
    NftWithdraw(NftWithdraw),
    MtWithdraw(MtWithdraw),
}

impl From<FtWithdraw> for WithdrawEventMediator {
    fn from(w: FtWithdraw) -> Self {
        Self::FtWithdraw(w)
    }
}

impl From<NftWithdraw> for WithdrawEventMediator {
    fn from(w: NftWithdraw) -> Self {
        Self::NftWithdraw(w)
    }
}

impl From<MtWithdraw> for WithdrawEventMediator {
    fn from(w: MtWithdraw) -> Self {
        Self::MtWithdraw(w)
    }
}

impl<'a> From<WithdrawEventMediator> for DefuseEvent<'a> {
    fn from(ev: WithdrawEventMediator) -> Self {
        match ev {
            WithdrawEventMediator::FtWithdraw(w) => DefuseEvent::FtWithdraw(w),
            WithdrawEventMediator::NftWithdraw(w) => DefuseEvent::NftWithdraw(w),
            WithdrawEventMediator::MtWithdraw(w) => DefuseEvent::MtWithdraw(w),
        }
    }
}
