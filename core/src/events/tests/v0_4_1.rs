use defuse_serde_utils::base58::Base58;
use derive_more::derive::From;
use near_sdk::{CryptoHash, near, serde::Deserialize};
use serde_with::serde_as;
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
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct IntentEvent<T> {
    #[serde_as(as = "Base58")]
    pub intent_hash: CryptoHash,

    #[serde(flatten)]
    pub event: T,
}

// Defuse events according to defuse v0.4.1,
#[must_use = "make sure to `.emit()` this event"]
#[near(event_json(standard = "dip4"))]
#[derive(Debug, Clone, Deserialize, From)]
pub enum DefuseEventV0_4_1<'a> {
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
