use std::{borrow::Cow, collections::BTreeMap};

use near_contract_standards::non_fungible_token;
use near_sdk::{AccountId, AccountIdRef, CryptoHash, NearToken, json_types::U128, near};
use serde_with::{DisplayFromStr, serde_as};

use crate::{
    DefuseError, Result,
    accounts::AccountEvent,
    engine::{Engine, Inspector, State},
    events::DefuseEvent,
    tokens::Amounts,
};

use super::{ExecutableIntent, IntentEvent};

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Transfer a set of tokens from the signer to a specified account id, within the intents contract.
pub struct Transfer {
    pub receiver_id: AccountId,

    #[serde_as(as = "Amounts<BTreeMap<_, DisplayFromStr>>")]
    pub tokens: Amounts,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

impl ExecutableIntent for Transfer {
    fn execute_intent<S, I>(
        self,
        sender_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        if sender_id == self.receiver_id || self.tokens.is_empty() {
            return Err(DefuseError::InvalidIntent);
        }

        let event = DefuseEvent::Transfer(
            vec![IntentEvent::new(
                AccountEvent::new(sender_id, Cow::Borrowed(&self)),
                intent_hash,
            )]
            .into(),
        );
        engine.inspector.on_event(event);

        engine
            .state
            .internal_sub_balance(sender_id, self.tokens.clone())?;
        engine
            .state
            .internal_add_balance(self.receiver_id, self.tokens)?;
        Ok(())
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Withdraw given FT tokens from the intents contract to a given external account id (external being outside of intents).
pub struct FtWithdraw {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub amount: U128,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Message to pass to `ft_transfer_call`. Otherwise, `ft_transfer` will be used.
    /// NOTE: No refund will be made in case of insufficient `storage_deposit`
    /// on `token` for `receiver_id`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    /// Optionally make `storage_deposit` for `receiver_id` on `token`.
    /// The amount will be subtracted from user's NEP-141 `wNEAR` balance.
    /// NOTE: the `wNEAR` will not be refunded in case of fail
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_deposit: Option<NearToken>,
}

impl ExecutableIntent for FtWithdraw {
    #[inline]
    fn execute_intent<S, I>(
        self,
        owner_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        let event = DefuseEvent::FtWithdraw(Cow::Owned(vec![IntentEvent::new(
            AccountEvent::new(owner_id, Cow::Borrowed(&self)),
            intent_hash,
        )]));

        engine.inspector.on_event(event);

        engine.state.ft_withdraw(owner_id, self)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Withdraw given NFT tokens from the intents contract to a given external account id (external being outside of intents).
pub struct NftWithdraw {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub token_id: non_fungible_token::TokenId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Message to pass to `nft_transfer_call`. Otherwise, `nft_transfer` will be used.
    /// NOTE: No refund will be made in case of insufficient `storage_deposit`
    /// on `token` for `receiver_id`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    /// Optionally make `storage_deposit` for `receiver_id` on `token`.
    /// The amount will be subtracted from user's NEP-141 `wNEAR` balance.
    /// NOTE: the `wNEAR` will not be refunded in case of fail
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_deposit: Option<NearToken>,
}

impl ExecutableIntent for NftWithdraw {
    #[inline]
    fn execute_intent<S, I>(
        self,
        owner_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        let event = DefuseEvent::NftWithdraw(Cow::Owned(vec![IntentEvent::new(
            AccountEvent::new(owner_id, Cow::Borrowed(&self)),
            intent_hash,
        )]));

        engine.inspector.on_event(event);

        engine.state.nft_withdraw(owner_id, self)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Withdraw given MT tokens (i.e. [NEP-245](https://github.com/near/NEPs/blob/master/neps/nep-0245.md)) from the intents contract
/// to a given to an external account id (external being outside of intents).
///
/// If `msg` is given, `mt_batch_transfer_call()` will be used to transfer to the `receiver_id`. Otherwise, `mt_batch_transfer()` will be used.
pub struct MtWithdraw {
    pub token: AccountId,
    pub receiver_id: AccountId,
    pub token_ids: Vec<defuse_nep245::TokenId>,
    pub amounts: Vec<U128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Message to pass to `mt_batch_transfer_call`. Otherwise, `mt_batch_transfer` will be used.
    /// NOTE: No refund will be made in case of insufficient `storage_deposit`
    /// on `token` for `receiver_id`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,

    /// Optionally make `storage_deposit` for `receiver_id` on `token`.
    /// The amount will be subtracted from user's NEP-141 `wNEAR` balance.
    /// NOTE: the `wNEAR` will not be refunded in case of fail
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_deposit: Option<NearToken>,
}
impl ExecutableIntent for MtWithdraw {
    #[inline]
    fn execute_intent<S, I>(
        self,
        owner_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        let event = DefuseEvent::MtWithdraw(Cow::Owned(vec![IntentEvent::new(
            AccountEvent::new(owner_id, Cow::Borrowed(&self)),
            intent_hash,
        )]));

        engine.inspector.on_event(event);

        engine.state.mt_withdraw(owner_id, self)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Withdraw native tokens (NEAR) from the intents contract to a given external account id (external being outside of intents).
/// This will subtract from the account's wNEAR balance, and will be sent to the account specified as native NEAR.
/// NOTE: the `wNEAR` will not be refunded in case of fail (e.g. `receiver_id`
/// account does not exist).
pub struct NativeWithdraw {
    pub receiver_id: AccountId,
    pub amount: NearToken,
}

impl ExecutableIntent for NativeWithdraw {
    #[inline]
    fn execute_intent<S, I>(
        self,
        owner_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        let event = DefuseEvent::NativeWithdraw(Cow::Owned(vec![IntentEvent::new(
            AccountEvent::new(owner_id, Cow::Borrowed(&self)),
            intent_hash,
        )]));

        engine.inspector.on_event(event);

        engine.state.native_withdraw(owner_id, self)
    }
}

/// Make [NEP-145](https://nomicon.io/Standards/StorageManagement#nep-145)
/// `storage_deposit` for an `account_id` on `contract_id`.
/// The `amount` will be subtracted from user's NEP-141 `wNEAR` balance.
/// NOTE: the `wNEAR` will not be refunded in any case.
///
/// WARNING: use this intent only if paying storage_deposit is not a prerequisite
/// for other intents to succeed. If some intent (e.g. ft_withdraw) requires storage_deposit,
/// then use storage_deposit field of corresponding intent instead of adding a separate
/// `StorageDeposit` intent. This is due to the fact that intents that fire `Promise`s
/// are not guaranteed to be executed sequentially, in the order of the provided intents in
/// `DefuseIntents`.
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct StorageDeposit {
    pub contract_id: AccountId,
    pub account_id: AccountId,
    pub amount: NearToken,
}

impl ExecutableIntent for StorageDeposit {
    #[inline]
    fn execute_intent<S, I>(
        self,
        owner_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        let event = DefuseEvent::StorageDeposit(Cow::Owned(vec![IntentEvent::new(
            AccountEvent::new(owner_id, Cow::Borrowed(&self)),
            intent_hash,
        )]));

        engine.inspector.on_event(event);
        engine.state.storage_deposit(owner_id, self)
    }
}
