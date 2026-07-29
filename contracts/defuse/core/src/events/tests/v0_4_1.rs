use derive_more::derive::From;
use serde::Deserialize;
use serde::Serialize;
use serde_with::{base58::Base58, serde_as};
use std::borrow::Cow;

use crate::{
    accounts::{AccountEvent, NonceEvent, PublicKeyEvent, SaltRotationEvent},
    fees::{FeeChangedEvent, FeeCollectorChangedEvent},
    intents::{
        account::SetAuthByPredecessorId,
        token_diff::TokenDiffEvent,
        tokens::{FtWithdraw, MtWithdraw, NativeWithdraw, NftWithdraw, StorageDeposit},
    },
    tokens::TransferEvent,
};

#[must_use = "make sure to `.emit()` this event"]
#[serde_as]
#[cfg_attr(feature = "abi", derive(::schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentEvent<T> {
    #[serde_as(as = "Base58")]
    pub intent_hash: [u8; 32],

    #[serde(flatten)]
    pub event: T,
}

// Defuse events according to defuse v0.4.1,
#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(feature = "runtime", ::near_sdk::near(event_json(standard = "dip4")))]
#[derive(Debug, Clone, Deserialize, From)]
pub enum DefuseEventV0_4_1<'a> {
    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    #[from(skip)]
    PublicKeyAdded(AccountEvent<'a, PublicKeyEvent<'a>>),
    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    #[from(skip)]
    PublicKeyRemoved(AccountEvent<'a, PublicKeyEvent<'a>>),

    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    FeeChanged(FeeChangedEvent),
    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    FeeCollectorChanged(FeeCollectorChangedEvent<'a>),

    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    Transfer(Cow<'a, [IntentEvent<AccountEvent<'a, TransferEvent<'a>>>]>),

    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    TokenDiff(Cow<'a, [IntentEvent<AccountEvent<'a, TokenDiffEvent<'a>>>]>),

    #[cfg_attr(feature = "runtime", event_version("0.3.1"))]
    IntentsExecuted(Cow<'a, [IntentEvent<AccountEvent<'a, NonceEvent>>]>),

    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    FtWithdraw(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, FtWithdraw>>>]>),

    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    NftWithdraw(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, NftWithdraw>>>]>),

    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    MtWithdraw(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, MtWithdraw>>>]>),

    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    NativeWithdraw(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, NativeWithdraw>>>]>),

    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    StorageDeposit(Cow<'a, [IntentEvent<AccountEvent<'a, Cow<'a, StorageDeposit>>>]>),

    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    #[from(skip)]
    AccountLocked(AccountEvent<'a, ()>),
    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    #[from(skip)]
    AccountUnlocked(AccountEvent<'a, ()>),

    #[cfg_attr(feature = "runtime", event_version("0.3.0"))]
    SetAuthByPredecessorId(AccountEvent<'a, SetAuthByPredecessorId>),

    #[cfg_attr(feature = "runtime", event_version("0.4.0"))]
    SaltRotation(SaltRotationEvent),
}
