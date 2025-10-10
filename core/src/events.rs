use std::borrow::Cow;

use derive_more::derive::From;
use near_sdk::{near, serde::Deserialize};

use crate::{
    accounts::{AccountEvent, NonceEvent, PublicKeyEvent},
    fees::{FeeChangedEvent, FeeCollectorChangedEvent},
    intents::{
        IntentEvent, IntoStaticIntentEvent,
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

    #[event_version("0.3.1")]
    IntentsExecuted(Cow<'a, [IntentEvent<AccountEvent<'a, NonceEvent>>]>),

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

impl<'a> DefuseEvent<'a> {
    /// Helper function to convert event slices to 'static using IntoStaticIntentEvent trait
    #[inline]
    fn convert_events<E>(events: Cow<'a, [E]>) -> Cow<'static, [E::Output]>
    where
        E: IntoStaticIntentEvent + Clone,
        E::Output: Clone,
        [E]: ToOwned<Owned = Vec<E>>,
    {
        events
            .into_owned()
            .into_iter()
            .map(IntoStaticIntentEvent::into_static)
            .collect::<Vec<_>>()
            .into()
    }

    /// Convert this event into one with a `'static` lifetime by converting
    /// all borrowed data (`Cow::Borrowed`) into owned data (`Cow::Owned`).
    pub fn into_static(self) -> DefuseEvent<'static> {
        match self {
            DefuseEvent::PublicKeyAdded(event) => {
                DefuseEvent::PublicKeyAdded(event.into_owned_public_key())
            }
            DefuseEvent::PublicKeyRemoved(event) => {
                DefuseEvent::PublicKeyRemoved(event.into_owned_public_key())
            }
            DefuseEvent::FeeChanged(event) => DefuseEvent::FeeChanged(event),
            DefuseEvent::FeeCollectorChanged(event) => {
                DefuseEvent::FeeCollectorChanged(event.into_owned())
            }
            DefuseEvent::Transfer(events) => {
                DefuseEvent::Transfer(Self::convert_events(events))
            }
            DefuseEvent::TokenDiff(events) => {
                DefuseEvent::TokenDiff(Self::convert_events(events))
            }
            DefuseEvent::IntentsExecuted(events) => {
                DefuseEvent::IntentsExecuted(Self::convert_events(events))
            }
            DefuseEvent::FtWithdraw(events) => {
                DefuseEvent::FtWithdraw(Self::convert_events(events))
            }
            DefuseEvent::NftWithdraw(events) => {
                DefuseEvent::NftWithdraw(Self::convert_events(events))
            }
            DefuseEvent::MtWithdraw(events) => {
                DefuseEvent::MtWithdraw(Self::convert_events(events))
            }
            DefuseEvent::NativeWithdraw(events) =>
                DefuseEvent::NativeWithdraw(Self::convert_events(events))
            ,
            DefuseEvent::StorageDeposit(events) =>
                DefuseEvent::StorageDeposit(Self::convert_events(events))
            ,
            DefuseEvent::AccountLocked(event) => DefuseEvent::AccountLocked(event.into_owned()),
            DefuseEvent::AccountUnlocked(event) => DefuseEvent::AccountUnlocked(event.into_owned()),
            DefuseEvent::SetAuthByPredecessorId(event) => {
                DefuseEvent::SetAuthByPredecessorId(event.into_owned())
            }
        }
    }
}

pub trait DefuseIntentEmit<'a>: Into<DefuseEvent<'a>> {
    #[inline]
    fn emit(self) {
        DefuseEvent::emit(&self.into());
    }
}

impl<'a, T> DefuseIntentEmit<'a> for T where T: Into<DefuseEvent<'a>> {}
