use std::borrow::Cow;

use derive_more::derive::From;
use near_sdk::{near, serde::Deserialize};

use crate::{
    accounts::{AccountEvent, PublicKeyEvent},
    fees::{FeeChangedEvent, FeeCollectorChangedEvent},
    intents::{
        IntentEvent,
        token_diff::TokenDiffEvent,
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit, Transfer},
    },
};

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "dip4"))]
#[derive(Debug, Clone, Deserialize, From)]
// FIXME: Check with FE if it's OK to add arms + update all arm versions to be the same new version
pub enum DefuseEvent<'a> {
    #[event_version("0.2.1")]
    #[from(skip)]
    PublicKeyAdded(AccountEvent<'a, PublicKeyEvent<'a>>),
    #[event_version("0.2.1")]
    #[from(skip)]
    PublicKeyRemoved(AccountEvent<'a, PublicKeyEvent<'a>>),

    #[event_version("0.2.1")]
    FeeChanged(FeeChangedEvent),
    #[event_version("0.2.1")]
    FeeCollectorChanged(FeeCollectorChangedEvent<'a>),

    #[event_version("0.2.1")]
    Transfer(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, Transfer>>>]>),

    #[event_version("0.2.1")]
    TokenDiff(Cow<'a, [IntentEvent<AccountEvent<'a, TokenDiffEvent<'a>>>]>),

    #[event_version("0.2.1")]
    IntentsExecuted(Cow<'a, [IntentEvent<AccountEvent<'a, ()>>]>),

    #[event_version("0.2.1")]
    FtWithdraw(FtWithdraw),

    #[event_version("0.2.1")]
    NftWithdraw(NftWithdraw),

    #[event_version("0.2.1")]
    MtWithdraw(MtWithdraw),

    #[event_version("0.2.1")]
    NativeWithdraw(NativeWithdraw),

    #[event_version("0.2.1")]
    StorageDeposit(StorageDeposit),
}

pub trait DefuseIntentEmit<'a>: Into<DefuseEvent<'a>> {
    #[inline]
    fn emit(self) {
        DefuseEvent::emit(&self.into());
    }
}

impl<'a, T> DefuseIntentEmit<'a> for T where T: Into<DefuseEvent<'a>> {}
