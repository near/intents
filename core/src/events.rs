use std::borrow::Cow;

use derive_more::derive::From;
use near_sdk::{near, serde::Deserialize};

use crate::{
    accounts::{AccountEvent, PublicKeyEvent},
    fees::{FeeChangedEvent, FeeCollectorChangedEvent},
    intents::{
        IntentEvent,
        account::SetAuthByPredecessorId,
        token_diff::TokenDiffEvent,
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit, Transfer},
    },
};

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "dip4"))]
#[derive(Debug, Clone, Deserialize, From)]
pub enum DefuseEvent<'a> {
    #[event_version("0.3.0")]
    #[from(skip)]
    PublicKeyAdded(AccountEvent<'a, PublicKeyEvent<'a>>),
    #[event_version("0.3.0")]
    #[from(skip)]
    PublicKeyRemoved(AccountEvent<'a, PublicKeyEvent<'a>>),

    #[event_version("0.3.0")]
    FeeChanged(FeeChangedEvent),
    #[event_version("0.3.0")]
    FeeCollectorChanged(FeeCollectorChangedEvent<'a>),

    #[event_version("0.3.0")]
    Transfer(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, Transfer>>>]>),

    #[event_version("0.3.0")]
    TokenDiff(Cow<'a, [IntentEvent<AccountEvent<'a, TokenDiffEvent<'a>>>]>),

    #[event_version("0.3.0")]
    IntentsExecuted(Cow<'a, [IntentEvent<AccountEvent<'a, ()>>]>),

    #[event_version("0.3.0")]
    FtWithdraw(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, FtWithdraw>>>]>),

    #[event_version("0.3.0")]
    NftWithdraw(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, NftWithdraw>>>]>),

    #[event_version("0.3.0")]
    MtWithdraw(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, MtWithdraw>>>]>),

    #[event_version("0.3.0")]
    NativeWithdraw(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, NativeWithdraw>>>]>),

    #[event_version("0.3.0")]
    StorageDeposit(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, StorageDeposit>>>]>),

    #[event_version("0.3.0")]
    #[from(skip)]
    AccountLocked(AccountEvent<'a, ()>),
    #[event_version("0.3.0")]
    #[from(skip)]
    AccountUnlocked(AccountEvent<'a, ()>),

    #[event_version("0.3.0")]
    SetAuthByPredecessorId(AccountEvent<'a, SetAuthByPredecessorId>),
}

pub trait DefuseIntentEmit<'a>: Into<DefuseEvent<'a>> {
    #[inline]
    fn emit(self) {
        DefuseEvent::emit(&self.into());
    }
}

impl<'a, T> DefuseIntentEmit<'a> for T where T: Into<DefuseEvent<'a>> {}
