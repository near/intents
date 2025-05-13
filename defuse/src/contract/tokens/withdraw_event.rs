use defuse_core::{
    events::DefuseEvent,
    intents::tokens::{FtWithdraw, MtWithdraw, NftWithdraw},
};
use near_sdk::near;

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub enum WithdrawEvent {
    FtWithdraw(FtWithdraw),
    NftWithdraw(NftWithdraw),
    MtWithdraw(MtWithdraw),
}

impl From<FtWithdraw> for WithdrawEvent {
    fn from(w: FtWithdraw) -> Self {
        Self::FtWithdraw(w)
    }
}

impl From<NftWithdraw> for WithdrawEvent {
    fn from(w: NftWithdraw) -> Self {
        Self::NftWithdraw(w)
    }
}

impl From<MtWithdraw> for WithdrawEvent {
    fn from(w: MtWithdraw) -> Self {
        Self::MtWithdraw(w)
    }
}

impl<'a> From<WithdrawEvent> for DefuseEvent<'a> {
    fn from(ev: WithdrawEvent) -> Self {
        match ev {
            WithdrawEvent::FtWithdraw(w) => DefuseEvent::FtWithdraw(w),
            WithdrawEvent::NftWithdraw(w) => DefuseEvent::NftWithdraw(w),
            WithdrawEvent::MtWithdraw(w) => DefuseEvent::MtWithdraw(w),
        }
    }
}
