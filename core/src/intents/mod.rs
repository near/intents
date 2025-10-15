pub mod account;
pub mod auth;
pub mod token_diff;
pub mod tokens;

use defuse_serde_utils::base58::Base58;
use derive_more::derive::From;
use near_sdk::{AccountIdRef, CryptoHash, near};
use serde_with::serde_as;
use tokens::{NativeWithdraw, StorageDeposit};

use crate::{
    Result,
    accounts::AccountEvent,
    engine::{Engine, Inspector, State},
    intents::{account::SetAuthByPredecessorId, auth::AuthCall},
};

use self::{
    account::{AddPublicKey, RemovePublicKey},
    token_diff::TokenDiff,
    tokens::{FtWithdraw, MtWithdraw, NftWithdraw, Transfer},
};

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct DefuseIntents {
    /// Sequence of intents to execute in given order. Empty list is also
    /// a valid sequence, i.e. it doesn't do anything, but still invalidates
    /// the `nonce` for the signer
    /// WARNING: Promises created by different intents are executed concurrently and does not rely on the order of the intents in this structure
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<Intent>,
}

#[near(serializers = [json])]
#[serde(tag = "intent", rename_all = "snake_case")]
#[derive(Debug, Clone, From)]
pub enum Intent {
    /// See [`AddPublicKey`]
    AddPublicKey(AddPublicKey),

    /// See [`RemovePublicKey`]
    RemovePublicKey(RemovePublicKey),

    /// See [`Transfer`]
    Transfer(Transfer),

    /// See [`FtWithdraw`]
    FtWithdraw(FtWithdraw),

    /// See [`NftWithdraw`]
    NftWithdraw(NftWithdraw),

    /// See [`MtWithdraw`]
    MtWithdraw(MtWithdraw),

    /// See [`NativeWithdraw`]
    NativeWithdraw(NativeWithdraw),

    /// See [`StorageDeposit`]
    StorageDeposit(StorageDeposit),

    /// See [`TokenDiff`]
    TokenDiff(TokenDiff),

    /// See [`SetAuthByPredecessorId`]
    SetAuthByPredecessorId(SetAuthByPredecessorId),

    /// See [`AuthCall`]
    AuthCall(AuthCall),
}

pub trait ExecutableIntent {
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector;
}

impl ExecutableIntent for DefuseIntents {
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        for intent in self.intents {
            intent.execute_intent(signer_id, engine, intent_hash)?;
        }
        Ok(())
    }
}

impl ExecutableIntent for Intent {
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        match self {
            Self::AddPublicKey(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::RemovePublicKey(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::Transfer(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::FtWithdraw(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::NftWithdraw(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::MtWithdraw(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::NativeWithdraw(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::StorageDeposit(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::TokenDiff(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::SetAuthByPredecessorId(intent) => {
                intent.execute_intent(signer_id, engine, intent_hash)
            }
            Self::AuthCall(intent) => intent.execute_intent(signer_id, engine, intent_hash),
        }
    }
}

#[must_use = "make sure to `.emit()` this event"]
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentEvent<T> {
    #[serde_as(as = "Base58")]
    pub intent_hash: CryptoHash,

    #[serde(flatten)]
    pub event: T,
}

/// Trait for converting borrowed `IntentEvent` types to owned ('static) versions
pub trait IntoStaticIntentEvent {
    type Output;
    fn into_static(self) -> Self::Output;
}

// For IntentEvent with Cow-wrapped event types
impl<'a, T> IntoStaticIntentEvent for IntentEvent<AccountEvent<'a, std::borrow::Cow<'a, T>>>
where
    T: ToOwned + ?Sized + 'static,
    T::Owned: 'static,
{
    #[allow(clippy::use_self)] // False positive: Output has 'static lifetime, not 'a
    type Output = IntentEvent<AccountEvent<'static, std::borrow::Cow<'static, T>>>;

    #[inline]
    fn into_static(self) -> Self::Output {
        IntentEvent {
            intent_hash: self.intent_hash,
            event: AccountEvent {
                account_id: std::borrow::Cow::Owned(self.event.account_id.into_owned()),
                event: std::borrow::Cow::Owned(self.event.event.into_owned()),
            },
        }
    }
}

// For IntentEvent with TokenDiffEvent
impl<'a> IntoStaticIntentEvent for IntentEvent<AccountEvent<'a, token_diff::TokenDiffEvent<'a>>> {
    #[allow(clippy::use_self)] // False positive: Output has 'static lifetime, not 'a
    type Output = IntentEvent<AccountEvent<'static, token_diff::TokenDiffEvent<'static>>>;

    #[inline]
    fn into_static(self) -> Self::Output {
        IntentEvent {
            intent_hash: self.intent_hash,
            event: self.event.into_owned_token_diff(),
        }
    }
}

// For IntentEvent with NonceEvent specifically
impl IntoStaticIntentEvent for IntentEvent<AccountEvent<'_, crate::accounts::NonceEvent>> {
    #[allow(clippy::use_self)] // False positive: Output has 'static lifetime, not '_
    type Output = IntentEvent<AccountEvent<'static, crate::accounts::NonceEvent>>;

    #[inline]
    fn into_static(self) -> Self::Output {
        IntentEvent {
            intent_hash: self.intent_hash,
            event: AccountEvent {
                account_id: std::borrow::Cow::Owned(self.event.account_id.into_owned()),
                event: self.event.event,
            },
        }
    }
}

impl<T> IntentEvent<T> {
    #[inline]
    pub const fn new(event: T, intent_hash: CryptoHash) -> Self {
        Self { intent_hash, event }
    }
}
