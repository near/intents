pub mod account;
pub mod token_diff;
pub mod tokens;

use defuse_serde_utils::base58::Base58;
use derive_more::derive::From;
use near_sdk::{near, AccountIdRef, CryptoHash};
use serde_with::serde_as;
use tokens::NativeWithdraw;

use crate::{
    engine::{Engine, Inspector, State},
    Result,
};

use self::{
    account::{AddPublicKey, InvalidateNonces, RemovePublicKey},
    token_diff::TokenDiff,
    tokens::{FtWithdraw, MtBatchTransfer, MtWithdraw, NftWithdraw},
};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct DefuseIntents {
    /// Sequence of intents to execute in given order. Empty list is also
    /// a valid sequence, i.e. it doesn't do anything, but still invalidates
    /// the `nonce` for the signer
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<Intent>,
}

#[near(serializers = [borsh, json])]
#[serde(tag = "intent", rename_all = "snake_case")]
#[derive(Debug, Clone, From)]
pub enum Intent {
    AddPublicKey(AddPublicKey),
    RemovePublicKey(RemovePublicKey),
    InvalidateNonces(InvalidateNonces),

    MtBatchTransfer(MtBatchTransfer),

    FtWithdraw(FtWithdraw),
    NftWithdraw(NftWithdraw),
    MtWithdraw(MtWithdraw),
    NativeWithdraw(NativeWithdraw),

    TokenDiff(TokenDiff),
}

pub trait ExecutableIntent {
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
    ) -> Result<()>
    where
        S: State,
        I: Inspector;
}

impl ExecutableIntent for DefuseIntents {
    fn execute_intent<S, I>(self, signer_id: &AccountIdRef, engine: &mut Engine<S, I>) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        for intent in self.intents {
            intent.execute_intent(signer_id, engine)?;
        }
        Ok(())
    }
}

impl ExecutableIntent for Intent {
    fn execute_intent<S, I>(self, signer_id: &AccountIdRef, engine: &mut Engine<S, I>) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        match self {
            Self::AddPublicKey(intent) => intent.execute_intent(signer_id, engine),
            Self::RemovePublicKey(intent) => intent.execute_intent(signer_id, engine),
            Self::InvalidateNonces(intent) => intent.execute_intent(signer_id, engine),
            Self::MtBatchTransfer(intent) => intent.execute_intent(signer_id, engine),
            Self::FtWithdraw(intent) => intent.execute_intent(signer_id, engine),
            Self::NftWithdraw(intent) => intent.execute_intent(signer_id, engine),
            Self::MtWithdraw(intent) => intent.execute_intent(signer_id, engine),
            Self::NativeWithdraw(intent) => intent.execute_intent(signer_id, engine),
            Self::TokenDiff(intent) => intent.execute_intent(signer_id, engine),
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
pub struct IntentExecutedEvent {
    #[serde_as(as = "Base58")]
    pub hash: CryptoHash,
}
