use derive_more::derive::From;
use serde::Deserialize;
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

#[cfg_attr(
    not(feature = "near-contract"),
    must_use,
    derive(::serde::Serialize),
    serde(tag = "event", content = "data", rename_all = "snake_case")
)]
#[cfg_attr(
    feature = "near-contract",
    must_use = "make sure to `.emit()` this event",
    ::near_sdk::near(event_json(standard = "dip4"))
)]
#[derive(Debug, Clone, Deserialize, From)]
pub enum DefuseEvent<'a> {
    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    #[from(skip)]
    PublicKeyAdded(MaybeIntentEvent<AccountEvent<'a, PublicKeyEvent<'a>>>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    #[from(skip)]
    PublicKeyRemoved(MaybeIntentEvent<AccountEvent<'a, PublicKeyEvent<'a>>>),

    #[cfg_attr(feature = "near-contract", event_version("0.3.0"))]
    FeeChanged(FeeChangedEvent),
    #[cfg_attr(feature = "near-contract", event_version("0.3.0"))]
    FeeCollectorChanged(FeeCollectorChangedEvent<'a>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    Transfer(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, TransferEvent<'a>>>]>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    TokenDiff(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, TokenDiffEvent<'a>>>]>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    IntentsExecuted(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, NonceEvent>>]>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    FtWithdraw(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, FtWithdraw>>>]>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    NftWithdraw(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, NftWithdraw>>>]>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    MtWithdraw(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, MtWithdraw>>>]>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    NativeWithdraw(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, NativeWithdraw>>>]>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    StorageDeposit(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, StorageDeposit>>>]>),

    #[cfg(feature = "imt")]
    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    ImtMint(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, ImtMintEvent<'a>>>]>),
    #[cfg(feature = "imt")]
    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    ImtBurn(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, Cow<'a, ImtBurn>>>]>),

    #[cfg_attr(feature = "near-contract", event_version("0.3.0"))]
    #[from(skip)]
    AccountLocked(AccountEvent<'a, ()>),
    #[cfg_attr(feature = "near-contract", event_version("0.3.0"))]
    #[from(skip)]
    AccountUnlocked(AccountEvent<'a, ()>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.3"))]
    SetAuthByPredecessorId(MaybeIntentEvent<AccountEvent<'a, Cow<'a, SetAuthByPredecessorId>>>),

    #[cfg_attr(feature = "near-contract", event_version("0.4.0"))]
    SaltRotation(SaltRotationEvent),
}

#[cfg(feature = "near-contract")]
pub trait DefuseIntentEmit<'a>: Into<DefuseEvent<'a>> {
    #[inline]
    fn emit(self) {
        DefuseEvent::emit(&self.into());
    }
}

#[cfg(feature = "near-contract")]
impl<'a, T> DefuseIntentEmit<'a> for T where T: Into<DefuseEvent<'a>> {}

#[cfg(test)]
mod tests;
