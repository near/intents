use anyhow::Result;
use defuse_nep245::{Token, TokenId};
use near_account_id::AccountId;
use near_kit::{Final, Near, NearToken};
use serde::{Deserialize, Serialize};

use crate::{U128, extensions::DEFAULT_GAS, outcome::SuccessfulExecutionOutcome};

#[derive(Serialize, Deserialize)]
pub struct MtTokenArgs {
    pub token_ids: Vec<TokenId>,
}

#[derive(Serialize, Deserialize)]
pub struct MtBalanceOfArgs {
    pub account_id: AccountId,
    pub token_id: TokenId,
}

#[derive(Serialize, Deserialize)]
pub struct MtBatchBalanceOfArgs {
    pub account_id: AccountId,
    pub token_ids: Vec<TokenId>,
}

#[derive(Serialize, Deserialize)]
pub struct MtSupplyArgs {
    pub token_id: TokenId,
}

#[derive(Serialize, Deserialize)]
pub struct MtBatchSupplyArgs {
    pub token_ids: Vec<TokenId>,
}

#[derive(Serialize, Deserialize)]
pub struct MtTransferArgs {
    pub receiver_id: AccountId,
    pub token_id: TokenId,
    pub amount: U128,
    pub approval: Option<(AccountId, u64)>,
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct MtBatchTransferArgs {
    pub receiver_id: AccountId,
    pub token_ids: Vec<TokenId>,
    pub amounts: Vec<U128>,
    pub approvals: Option<Vec<Option<(AccountId, u64)>>>,
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct MtTransferCallArgs {
    pub receiver_id: AccountId,
    pub token_id: TokenId,
    pub amount: U128,
    pub approval: Option<(AccountId, u64)>,
    pub memo: Option<String>,
    pub msg: String,
}

#[derive(Serialize, Deserialize)]
pub struct MtBatchTransferCallArgs {
    pub receiver_id: AccountId,
    pub token_ids: Vec<TokenId>,
    pub amounts: Vec<U128>,
    pub approvals: Option<Vec<Option<(AccountId, u64)>>>,
    pub memo: Option<String>,
    pub msg: String,
}

// TODO: may be also make ext helpers for view methods?
#[near_kit::contract]
pub trait Mt {
    fn mt_token(&self, args: MtTokenArgs) -> Vec<Option<Token>>;
    fn mt_balance_of(&self, args: MtBalanceOfArgs) -> U128;
    fn mt_batch_balance_of(&self, args: MtBatchBalanceOfArgs) -> Vec<U128>;
    fn mt_supply(&self, args: MtSupplyArgs) -> Option<U128>;
    fn mt_batch_supply(&self, args: MtBatchSupplyArgs) -> Vec<Option<U128>>;

    #[call]
    fn mt_transfer(&mut self, args: MtTransferArgs);

    #[call]
    fn mt_batch_transfer(&mut self, args: MtBatchTransferArgs);

    #[call]
    fn mt_transfer_call(&mut self, args: MtTransferCallArgs);

    #[call]
    fn mt_batch_transfer_call(&mut self, args: MtBatchTransferCallArgs);
}

pub trait MtExt {
    async fn mt_transfer(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_id: impl Into<TokenId>,
        amount: u128,
        approval: impl Into<Option<(AccountId, u64)>>,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn mt_batch_transfer(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        approvals: impl Into<Option<Vec<Option<(AccountId, u64)>>>>,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn mt_transfer_call(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_id: impl Into<TokenId>,
        amount: u128,
        approval: impl Into<Option<(AccountId, u64)>>,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn mt_batch_transfer_call(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        approvals: impl Into<Option<Vec<Option<(AccountId, u64)>>>>,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl MtExt for Near {
    async fn mt_transfer(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_id: impl Into<TokenId>,
        amount: u128,
        approval: impl Into<Option<(AccountId, u64)>>,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(contract.into())
            .add_action(
                Mt::mt_transfer(MtTransferArgs {
                    receiver_id: receiver_id.into(),
                    token_id: token_id.into(),
                    amount: amount.into(),
                    approval: approval.into(),
                    memo: memo.into(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn mt_batch_transfer(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        approvals: impl Into<Option<Vec<Option<(AccountId, u64)>>>>,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(contract.into())
            .add_action(
                Mt::mt_batch_transfer(MtBatchTransferArgs {
                    receiver_id: receiver_id.into(),
                    token_ids: token_ids.into_iter().map(Into::into).collect(),
                    amounts: amounts.into_iter().map(Into::into).collect(),
                    approvals: approvals.into(),
                    memo: memo.into(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn mt_transfer_call(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_id: impl Into<TokenId>,
        amount: u128,
        approval: impl Into<Option<(AccountId, u64)>>,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(contract.into())
            .add_action(
                Mt::mt_transfer_call(MtTransferCallArgs {
                    receiver_id: receiver_id.into(),
                    token_id: token_id.into(),
                    amount: amount.into(),
                    approval: approval.into(),
                    memo: memo.into(),
                    msg: msg.into(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn mt_batch_transfer_call(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl Into<AccountId>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        approvals: impl Into<Option<Vec<Option<(AccountId, u64)>>>>,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(contract.into())
            .add_action(
                Mt::mt_batch_transfer_call(MtBatchTransferCallArgs {
                    receiver_id: receiver_id.into(),
                    token_ids: token_ids.into_iter().map(Into::into).collect(),
                    amounts: amounts.into_iter().map(Into::into).collect(),
                    approvals: approvals.into(),
                    memo: memo.into(),
                    msg: msg.into(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }
}
