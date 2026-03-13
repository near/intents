use derive_more::derive::From;
use near_sdk::{near, serde::Deserialize};
use std::borrow::Cow;

use crate::{
    accounts::{AccountEvent, NonceEvent, PublicKeyEvent, SaltRotationEvent},
    fees::{FeeChangedEvent, FeeCollectorChangedEvent},
    intents::{
        MaybeIntentEvent,
        account::SetAuthByPredecessorId,
        token_diff::TokenDiffEvent,
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit},
    },
    tokens::TransferEvent,
};

#[cfg(feature = "imt")]
use crate::{intents::imt::ImtBurn, tokens::imt::ImtMintEvent};

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "dip4"))]
#[derive(Debug, Clone, Deserialize, From)]
pub enum DefuseEvent<'a> {
    #[event_version("0.4.3")]
    #[from(skip)]
    PublicKeyAdded(MaybeIntentEvent<AccountEvent<'a, PublicKeyEvent<'a>>>),
    #[event_version("0.4.3")]
    #[from(skip)]
    PublicKeyRemoved(MaybeIntentEvent<AccountEvent<'a, PublicKeyEvent<'a>>>),

    #[event_version("0.3.0")]
    FeeChanged(FeeChangedEvent),
    #[event_version("0.3.0")]
    FeeCollectorChanged(FeeCollectorChangedEvent<'a>),

    #[event_version("0.4.3")]
    Transfer(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, TransferEvent<'a>>>]>),

    #[event_version("0.4.3")]
    TokenDiff(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, TokenDiffEvent<'a>>>]>),

    #[event_version("0.4.3")]
    IntentsExecuted(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, NonceEvent>>]>),

    #[event_version("0.4.3")]
    FtWithdraw(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, FtWithdraw>>>]>),

    #[event_version("0.4.3")]
    NftWithdraw(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, NftWithdraw>>>]>),

    #[event_version("0.4.3")]
    MtWithdraw(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, MtWithdraw>>>]>),

    #[event_version("0.4.3")]
    NativeWithdraw(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, NativeWithdraw>>>]>),

    #[event_version("0.4.3")]
    StorageDeposit(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, StorageDeposit>>>]>),

    #[cfg(feature = "imt")]
    #[event_version("0.4.3")]
    ImtMint(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, ImtMintEvent<'a>>>]>),
    #[cfg(feature = "imt")]
    #[event_version("0.4.3")]
    ImtBurn(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, ImtBurn>>>]>),

    #[event_version("0.3.0")]
    #[from(skip)]
    AccountLocked(AccountEvent<'a, ()>),
    #[event_version("0.3.0")]
    #[from(skip)]
    AccountUnlocked(AccountEvent<'a, ()>),

    #[event_version("0.4.3")]
    SetAuthByPredecessorId(MaybeIntentEvent<AccountEvent<'a, Cow<'a, SetAuthByPredecessorId>>>),

    #[event_version("0.4.0")]
    SaltRotation(SaltRotationEvent),
}

pub trait DefuseIntentEmit<'a>: Into<DefuseEvent<'a>> {
    #[inline]
    fn emit(self) {
        DefuseEvent::emit(&self.into());
    }
}

impl<'a, T> DefuseIntentEmit<'a> for T where T: Into<DefuseEvent<'a>> {}

#[cfg(test)]
mod tests;
