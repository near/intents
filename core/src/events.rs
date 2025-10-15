use std::borrow::Cow;

use defuse_nep245::MtEvent;
use derive_more::derive::From;
use near_sdk::{near, serde::Deserialize};

use crate::{
    accounts::{AccountEvent, NonceEvent, PublicKeyEvent, SaltRotationEvent},
    fees::{FeeChangedEvent, FeeCollectorChangedEvent},
    intents::{
        IntentEvent, IntoStaticIntentEvent,
        account::SetAuthByPredecessorId,
        token_diff::TokenDiffEvent,
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit, Transfer},
    },
};

#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefuseEvent<'a> {
    Dip4Event(Dip4Event<'a>),
    Nep245Event(MtEvent<'a>),
}

impl DefuseEvent<'_> {
    pub fn emit(self) {
        match self {
            DefuseEvent::Dip4Event(defuse_event) => defuse_event.emit(),
            DefuseEvent::Nep245Event(mt_event) => mt_event.emit(),
        }
    }
}

impl DefuseEvent<'_> {
    pub fn into_owned(self) -> DefuseEvent<'static> {
        match self {
            DefuseEvent::Dip4Event(event) => DefuseEvent::Dip4Event(event.into_owned()),
            DefuseEvent::Nep245Event(event) => DefuseEvent::Nep245Event(event.into_owned()),
        }
    }
}

impl<'a> From<Dip4Event<'a>> for DefuseEvent<'a> {
    fn from(ev: Dip4Event<'a>) -> Self {
        DefuseEvent::Dip4Event(ev)
    }
}

impl<'a> From<MtEvent<'a>> for DefuseEvent<'a> {
    fn from(ev: MtEvent<'a>) -> Self {
        DefuseEvent::Nep245Event(ev)
    }
}

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "dip4"))]
#[derive(Debug, Clone, Deserialize, From, PartialEq, Eq)]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    derive(schemars::JsonSchema)
)]
pub enum Dip4Event<'a> {
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

    #[event_version("0.4.0")]
    SaltRotation(SaltRotationEvent),
}

impl<'a> Dip4Event<'a> {
    /// Helper function to convert event slices to 'static using `IntoStaticIntentEvent` trait
    #[inline]
    pub fn convert_events<E>(events: Cow<'a, [E]>) -> Cow<'static, [E::Output]>
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
    pub fn into_owned(self) -> Dip4Event<'static> {
        match self {
            Dip4Event::PublicKeyAdded(event) => {
                Dip4Event::PublicKeyAdded(event.into_owned_public_key())
            }
            Dip4Event::PublicKeyRemoved(event) => {
                Dip4Event::PublicKeyRemoved(event.into_owned_public_key())
            }
            Dip4Event::FeeChanged(event) => Dip4Event::FeeChanged(event),
            Dip4Event::FeeCollectorChanged(event) => {
                Dip4Event::FeeCollectorChanged(event.into_owned())
            }
            Dip4Event::Transfer(events) => Dip4Event::Transfer(Self::convert_events(events)),
            Dip4Event::TokenDiff(events) => Dip4Event::TokenDiff(Self::convert_events(events)),
            Dip4Event::IntentsExecuted(events) => {
                Dip4Event::IntentsExecuted(Self::convert_events(events))
            }
            Dip4Event::FtWithdraw(events) => Dip4Event::FtWithdraw(Self::convert_events(events)),
            Dip4Event::NftWithdraw(events) => Dip4Event::NftWithdraw(Self::convert_events(events)),
            Dip4Event::MtWithdraw(events) => Dip4Event::MtWithdraw(Self::convert_events(events)),
            Dip4Event::NativeWithdraw(events) => {
                Dip4Event::NativeWithdraw(Self::convert_events(events))
            }
            Dip4Event::StorageDeposit(events) => {
                Dip4Event::StorageDeposit(Self::convert_events(events))
            }
            Dip4Event::AccountLocked(event) => Dip4Event::AccountLocked(event.into_owned()),
            Dip4Event::AccountUnlocked(event) => Dip4Event::AccountUnlocked(event.into_owned()),
            Dip4Event::SetAuthByPredecessorId(event) => {
                Dip4Event::SetAuthByPredecessorId(event.into_owned())
            }
            Dip4Event::SaltRotation(event) => Dip4Event::SaltRotation(event),
        }
    }
}
