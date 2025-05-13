use defuse_core::{
    events::DefuseEvent,
    intents::tokens::{FtWithdraw, MtWithdraw, NftWithdraw},
};

/// A withdraw is done through a unified function using the `MultiToken` interface.
/// This struct helps in transferring the withdraw event information from the
/// type-specific withdraw (Ft/Mt/Nft, etc), to the unified part that performs
/// the withdrawal.
#[derive(Debug, Clone)]
pub enum WithdrawEventMediator {
    Ft(FtWithdraw),
    Nft(NftWithdraw),
    Mt(MtWithdraw),
}

impl From<FtWithdraw> for WithdrawEventMediator {
    fn from(w: FtWithdraw) -> Self {
        Self::Ft(w)
    }
}

impl From<NftWithdraw> for WithdrawEventMediator {
    fn from(w: NftWithdraw) -> Self {
        Self::Nft(w)
    }
}

impl From<MtWithdraw> for WithdrawEventMediator {
    fn from(w: MtWithdraw) -> Self {
        Self::Mt(w)
    }
}

impl From<WithdrawEventMediator> for DefuseEvent<'_> {
    fn from(ev: WithdrawEventMediator) -> Self {
        match ev {
            WithdrawEventMediator::Ft(w) => DefuseEvent::FtWithdraw(w),
            WithdrawEventMediator::Nft(w) => DefuseEvent::NftWithdraw(w),
            WithdrawEventMediator::Mt(w) => DefuseEvent::MtWithdraw(w),
        }
    }
}
