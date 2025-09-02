pub mod account;
pub mod auth;
pub mod token_diff;
pub mod tokens;

use defuse_serde_utils::{base58::Base58, base64::Base64};
use derive_more::derive::From;
use near_sdk::{AccountIdRef, CryptoHash, near};
use serde_with::serde_as;
use tokens::{NativeWithdraw, StorageDeposit};

use crate::{
    Nonce, Result,
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
        nonce: Nonce,
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
        nonce: Nonce,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        for intent in self.intents {
            intent.execute_intent(signer_id, engine, intent_hash, nonce)?;
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
        nonce: Nonce,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        match self {
            Self::AddPublicKey(intent) => {
                intent.execute_intent(signer_id, engine, intent_hash, nonce)
            }
            Self::RemovePublicKey(intent) => {
                intent.execute_intent(signer_id, engine, intent_hash, nonce)
            }
            Self::Transfer(intent) => intent.execute_intent(signer_id, engine, intent_hash, nonce),
            Self::FtWithdraw(intent) => {
                intent.execute_intent(signer_id, engine, intent_hash, nonce)
            }
            Self::NftWithdraw(intent) => {
                intent.execute_intent(signer_id, engine, intent_hash, nonce)
            }
            Self::MtWithdraw(intent) => {
                intent.execute_intent(signer_id, engine, intent_hash, nonce)
            }
            Self::NativeWithdraw(intent) => {
                intent.execute_intent(signer_id, engine, intent_hash, nonce)
            }
            Self::StorageDeposit(intent) => {
                intent.execute_intent(signer_id, engine, intent_hash, nonce)
            }
            Self::TokenDiff(intent) => intent.execute_intent(signer_id, engine, intent_hash, nonce),
            Self::SetAuthByPredecessorId(intent) => {
                intent.execute_intent(signer_id, engine, intent_hash, nonce)
            }
            Self::AuthCall(intent) => intent.execute_intent(signer_id, engine, intent_hash, nonce),
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
#[derive(Debug, Clone)]
pub struct IntentEvent<T> {
    #[serde_as(as = "Base58")]
    pub intent_hash: CryptoHash,

    #[serde_as(as = "Base64")]
    pub nonce: Nonce,

    #[serde(flatten)]
    pub event: T,
}

impl<T> IntentEvent<T> {
    #[inline]
    pub const fn new(event: T, intent_hash: CryptoHash, nonce: Nonce) -> Self {
        Self {
            intent_hash,
            event,
            nonce,
        }
    }
}
