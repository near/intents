use defuse_core::{
    events::DefuseEvent,
    intents::tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit},
};

/// A token event (withdraw, storage deposit, etc) is done through a unified
/// function using the `MultiToken` interface.
/// This struct helps in transferring the withdraw event information from the
/// type-specific withdraw (Ft/Mt/Nft, etc), to the unified part that performs
/// the withdrawal.
#[derive(Debug, Clone)]
pub enum TokenEventMediator {
    Ft(FtWithdraw),
    Nft(NftWithdraw),
    Mt(MtWithdraw),
    NativeWithdraw(NativeWithdraw),
    StorageDeposit(StorageDeposit),
}

impl From<FtWithdraw> for TokenEventMediator {
    fn from(w: FtWithdraw) -> Self {
        Self::Ft(w)
    }
}

impl From<NftWithdraw> for TokenEventMediator {
    fn from(w: NftWithdraw) -> Self {
        Self::Nft(w)
    }
}

impl From<MtWithdraw> for TokenEventMediator {
    fn from(w: MtWithdraw) -> Self {
        Self::Mt(w)
    }
}

impl From<NativeWithdraw> for TokenEventMediator {
    fn from(w: NativeWithdraw) -> Self {
        Self::NativeWithdraw(w)
    }
}

impl From<StorageDeposit> for TokenEventMediator {
    fn from(s: StorageDeposit) -> Self {
        Self::StorageDeposit(s)
    }
}

impl From<TokenEventMediator> for DefuseEvent<'_> {
    fn from(ev: TokenEventMediator) -> Self {
        match ev {
            TokenEventMediator::Ft(w) => DefuseEvent::FtWithdraw(w),
            TokenEventMediator::Nft(w) => DefuseEvent::NftWithdraw(w),
            TokenEventMediator::Mt(w) => DefuseEvent::MtWithdraw(w),
            TokenEventMediator::NativeWithdraw(w) => DefuseEvent::NativeWithdraw(w),
            TokenEventMediator::StorageDeposit(s) => DefuseEvent::StorageDeposit(s),
        }
    }
}
