pub mod account;
pub mod auth;
pub mod token_diff;
pub mod tokens;

use derive_more::derive::From;
use near_sdk::{AccountIdRef, CryptoHash, near};
use tokens::{NativeWithdraw, StorageDeposit};

#[cfg(feature = "imt")]
use crate::intents::tokens::imt::{ImtBurn, ImtMint};

use crate::{
    Result,
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

    // See [`ImtMint`]
    #[cfg(feature = "imt")]
    ImtMint(ImtMint),

    // See [`ImtBurn`]
    #[cfg(feature = "imt")]
    ImtBurn(ImtBurn),
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
            #[cfg(feature = "imt")]
            Self::ImtMint(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            #[cfg(feature = "imt")]
            Self::ImtBurn(intent) => intent.execute_intent(signer_id, engine, intent_hash),
        }
    }
}
