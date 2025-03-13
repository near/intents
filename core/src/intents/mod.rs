pub mod account;
pub mod token_diff;
pub mod tokens;

use defuse_serde_utils::base58::Base58;
use derive_more::derive::From;
use near_sdk::{AccountIdRef, CryptoHash, near};
use serde_with::serde_as;
use tokens::NativeWithdraw;

use crate::{
    Result,
    engine::{Engine, Inspector, State},
};

use self::{
    account::{AddPublicKey, InvalidateNonces, RemovePublicKey},
    token_diff::TokenDiff,
    tokens::{FtWithdraw, MtWithdraw, NftWithdraw, Transfer},
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
    /// Given an account id, the user can add public keys. The added public keys can sign
    /// intents on behalf of these accounts, even to add new ones.
    /// Warning: Implicit account ids, by default, have their corresponding public keys added.
    /// Meaning: For a leaked private key, whose implicit account id had been used in intents,
    /// the user must manually rotate the underlying public key within intents.
    AddPublicKey(AddPublicKey),

    /// Remove the public key associated with a given account. See `AddPublicKey`.
    RemovePublicKey(RemovePublicKey),

    /// Every intent execution requires a nonce. This intent reserves a nonce for an account id,
    /// ensuring that a nonce won't be used multiple times. Note that an account id can have
    /// multiple nonces associated with it.
    InvalidateNonces(InvalidateNonces),

    /// Transfer a set of tokens from the signer to a specified account id, within the intents contract.
    Transfer(Transfer),

    /// Withdraw given FT tokens from the intents contract to a given external account id (external being outside of intents).
    FtWithdraw(FtWithdraw),

    /// Withdraw given NFT tokens from the intents contract to a given external account id (external being outside of intents).
    NftWithdraw(NftWithdraw),

    /// Withdraw given tokens (of any kind, under the MT standard) from the intents contract to a given
    /// external account id (external being outside of intents).
    MtWithdraw(MtWithdraw),

    /// Withdraw native tokens (NEAR) from the intents contract to a given external account id (external being outside of intents).
    NativeWithdraw(NativeWithdraw),

    /// The user declares the will to have a set of changes done to set of tokens. For example,
    /// a simple trade of 100 of token A for 200 of token B, can be represented by `TokenDiff`
    /// of {"A": -100, "B": 200} (this format is just for demonstration purposes).
    /// In general, the user can submit multiple changes with many tokens,
    /// not just token A for token B.
    TokenDiff(TokenDiff),
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
            Self::InvalidateNonces(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::Transfer(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::FtWithdraw(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::NftWithdraw(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::MtWithdraw(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::NativeWithdraw(intent) => intent.execute_intent(signer_id, engine, intent_hash),
            Self::TokenDiff(intent) => intent.execute_intent(signer_id, engine, intent_hash),
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
    #[serde(flatten)]
    pub event: T,
}

impl<T> IntentEvent<T> {
    #[inline]
    pub const fn new(event: T, intent_hash: CryptoHash) -> Self {
        Self { intent_hash, event }
    }
}
