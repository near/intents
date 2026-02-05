use std::borrow::Cow;

use derive_more::derive::From;
use near_sdk::{CryptoHash, near, serde::Deserialize};

use crate::{
    accounts::{AccountEvent, NonceEvent, PublicKeyEvent, SaltRotationEvent},
    fees::{FeeChangedEvent, FeeCollectorChangedEvent},
    intents::{
        IntentEvent,
        account::SetAuthByPredecessorId,
        token_diff::TokenDiffEvent,
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit},
    },
    tokens::TransferEvent,
};

#[cfg(feature = "imt")]
use crate::{intents::tokens::imt::ImtBurn, tokens::imt::ImtMintEvent};

#[must_use]
#[near(serializers = [json])]
#[serde(untagged)]
#[derive(Debug, Clone)]
pub enum ContractEvent<T> {
    Direct(T),
    Intent(IntentEvent<T>),
}

impl<T> MaybeIntentEvent<T> {
    #[inline]
    pub const fn direct(event: T) -> Self {
        Self(ContractEvent::Direct(event))
    }

    #[inline]
    pub const fn intent(event: T, intent_hash: CryptoHash) -> Self {
        Self(ContractEvent::Intent(IntentEvent::new(event, intent_hash)))
    }
}

/// Event that can be emitted either from a
/// function call or after intent execution
#[must_use = "make sure to `.emit()` this event"]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct MaybeIntentEvent<T>(pub ContractEvent<T>);

#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "dip4"))]
#[derive(Debug, Clone, Deserialize, From)]
pub enum DefuseEvent<'a> {
    #[event_version("0.3.0")]
    #[from(skip)]
    PublicKeyAdded(MaybeIntentEvent<AccountEvent<'a, PublicKeyEvent<'a>>>),
    #[event_version("0.3.0")]
    #[from(skip)]
    PublicKeyRemoved(MaybeIntentEvent<AccountEvent<'a, PublicKeyEvent<'a>>>),

    #[event_version("0.3.0")]
    FeeChanged(FeeChangedEvent),
    #[event_version("0.3.0")]
    FeeCollectorChanged(FeeCollectorChangedEvent<'a>),

    #[event_version("0.3.0")]
    Transfer(Cow<'a, [IntentEvent<AccountEvent<'a, TransferEvent<'a>>>]>),

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

    #[cfg(feature = "imt")]
    #[event_version("0.3.0")]
    ImtMint(Cow<'a, [MaybeIntentEvent<AccountEvent<'a, ImtMintEvent<'a>>>]>),

    #[cfg(feature = "imt")]
    #[event_version("0.3.0")]
    ImtBurn(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, ImtBurn>>>]>),
    #[event_version("0.3.0")]
    #[from(skip)]
    AccountLocked(AccountEvent<'a, ()>),
    #[event_version("0.3.0")]
    #[from(skip)]
    AccountUnlocked(AccountEvent<'a, ()>),

    #[event_version("0.3.0")]
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
