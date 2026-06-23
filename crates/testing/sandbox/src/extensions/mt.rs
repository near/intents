use anyhow::Result;
use core::str;
use defuse_nep245::{Token, TokenId};
use near_kit::{AccountId, AccountIdRef, Final, Near, NearToken};
use near_sdk::json_types::U128;
use serde::{Deserialize, Serialize};
use std::ops::RangeBounds;

use crate::{extensions::DEFAULT_GAS, outcome::SuccessfulExecutionOutcome};

#[derive(Serialize, Deserialize)]
pub struct MtTokenArgs {
    pub token_ids: Vec<TokenId>,
}

#[derive(Serialize)]
pub struct MtBalanceOfArgs<'a> {
    pub account_id: &'a AccountIdRef,
    pub token_id: &'a TokenId,
}

#[derive(Serialize)]
pub struct MtBatchBalanceOfArgs<'a> {
    pub account_id: &'a AccountIdRef,
    pub token_ids: &'a [TokenId],
}

#[derive(Serialize)]
pub struct MtSupplyArgs<'a> {
    pub token_id: &'a TokenId,
}

#[derive(Serialize)]
pub struct MtBatchSupplyArgs<'a> {
    pub token_ids: &'a [TokenId],
}

#[derive(Serialize)]
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

#[derive(Serialize, Deserialize)]
pub struct MtTokensArgs {
    pub from_index: Option<U128>,
    pub limit: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct MtTokensForOwnerArgs {
    pub account_id: AccountId,
    pub from_index: Option<U128>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct MtOnTransferArgs {
    pub sender_id: AccountId,
    pub previous_owner_ids: Vec<AccountId>,
    pub token_ids: Vec<TokenId>,
    pub amounts: Vec<U128>,
    pub msg: String,
}

#[near_kit::contract]
pub trait Mt {
    fn mt_token(&self, args: MtTokenArgs) -> Vec<Option<Token>>;
    fn mt_balance_of(&self, args: MtBalanceOfArgs) -> U128;
    fn mt_batch_balance_of(&self, args: MtBatchBalanceOfArgs) -> Vec<U128>;
    fn mt_supply(&self, args: MtSupplyArgs) -> Option<U128>;
    fn mt_batch_supply(&self, args: MtBatchSupplyArgs) -> Vec<Option<U128>>;

    fn mt_tokens(&self, args: MtTokensArgs) -> Vec<Token>;
    fn mt_tokens_for_owner(&self, args: MtTokensForOwnerArgs) -> Vec<Token>;

    #[call]
    fn mt_transfer(&mut self, args: MtTransferArgs);

    #[call]
    fn mt_batch_transfer(&mut self, args: MtBatchTransferArgs);

    #[call]
    fn mt_transfer_call(&mut self, args: MtTransferCallArgs);

    #[call]
    fn mt_batch_transfer_call(&mut self, args: MtBatchTransferCallArgs);

    #[call]
    fn mt_on_transfer(&mut self, args: MtOnTransferArgs);
}

pub trait MtExt {
    async fn mt_transfer(
        &self,
        contract: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl Into<TokenId>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn mt_batch_transfer(
        &self,
        contract: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn mt_transfer_call(
        &self,
        contract: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl Into<TokenId>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)>;

    async fn mt_batch_transfer_call(
        &self,
        contract: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)>;

    async fn mt_tokens(
        &self,
        contract: impl Into<AccountId>,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>>;

    async fn mt_tokens_for_owner(
        &self,
        contract: impl AsRef<AccountIdRef>,
        account_id: impl AsRef<AccountIdRef>,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>>;

    async fn mt_on_transfer(
        &self,
        defuse: impl AsRef<AccountIdRef>,
        args: MtOnTransferArgs,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<U128>)>;
}

impl MtExt for Near {
    async fn mt_transfer(
        &self,
        contract: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl Into<TokenId>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(contract.as_ref())
            .add_action(
                Mt::mt_transfer(MtTransferArgs {
                    receiver_id: receiver_id.as_ref().into(),
                    token_id: token_id.into(),
                    amount: amount.into(),
                    approval: None,
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
        contract: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(contract.as_ref())
            .add_action(
                Mt::mt_batch_transfer(MtBatchTransferArgs {
                    receiver_id: receiver_id.as_ref().into(),
                    token_ids: token_ids.into_iter().map(Into::into).collect(),
                    amounts: amounts.into_iter().map(Into::into).collect(),
                    approvals: None,
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
        contract: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl Into<TokenId>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)> {
        let outcome = self
            .transaction(contract.as_ref())
            .add_action(
                Mt::mt_transfer_call(MtTransferCallArgs {
                    receiver_id: receiver_id.as_ref().into(),
                    token_id: token_id.into(),
                    amount: amount.into(),
                    approval: None,
                    memo: memo.into(),
                    msg: msg.into(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?;

        let res = outcome.json::<Vec<U128>>()?;

        Ok((outcome.try_into()?, res.into_iter().map(|n| n.0).collect()))
    }

    async fn mt_batch_transfer_call(
        &self,
        contract: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)> {
        let outcome = self
            .transaction(contract.as_ref())
            .add_action(
                Mt::mt_batch_transfer_call(MtBatchTransferCallArgs {
                    receiver_id: receiver_id.as_ref().into(),
                    token_ids: token_ids.into_iter().map(Into::into).collect(),
                    amounts: amounts.into_iter().map(Into::into).collect(),
                    approvals: None,
                    memo: memo.into(),
                    msg: msg.into(),
                })
                .gas(DEFAULT_GAS)
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?;

        let res = outcome.json::<Vec<U128>>()?;

        Ok((outcome.try_into()?, res.into_iter().map(|n| n.0).collect()))
    }

    async fn mt_tokens(
        &self,
        contract: impl Into<AccountId>,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>> {
        let from = match range.start_bound() {
            std::ops::Bound::Included(v) => Some(*v),
            std::ops::Bound::Excluded(v) => Some(*v + 1),
            std::ops::Bound::Unbounded => None,
        };

        let to = match range.end_bound() {
            std::ops::Bound::Included(v) => Some(*v + 1),
            std::ops::Bound::Excluded(v) => Some(*v),
            std::ops::Bound::Unbounded => None,
        };

        let limit = match (from, to) {
            (Some(_) | None, None) => None,
            (None, Some(v)) => Some(v),
            (Some(f), Some(t)) => Some(t - f),
        };

        self.contract::<Mt>(contract.into())
            .mt_tokens(MtTokensArgs {
                from_index: from.map(|v| U128(v.try_into().unwrap())),
                limit,
            })
            .await
            .map_err(Into::into)
    }

    async fn mt_tokens_for_owner(
        &self,
        contract: impl AsRef<AccountIdRef>,
        account_id: impl AsRef<AccountIdRef>,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>> {
        let from = match range.start_bound() {
            std::ops::Bound::Included(v) => Some(*v),
            std::ops::Bound::Excluded(v) => Some(*v + 1),
            std::ops::Bound::Unbounded => None,
        };

        let to = match range.end_bound() {
            std::ops::Bound::Included(v) => Some(*v + 1),
            std::ops::Bound::Excluded(v) => Some(*v),
            std::ops::Bound::Unbounded => None,
        };

        let limit = match (from, to) {
            (Some(_) | None, None) => None,
            (None, Some(v)) => Some(v),
            (Some(f), Some(t)) => Some(t - f),
        };

        self.contract::<Mt>(contract.as_ref())
            .mt_tokens_for_owner(MtTokensForOwnerArgs {
                account_id: account_id.as_ref().into(),
                from_index: from.map(|v| U128(v.try_into().unwrap())),
                limit,
            })
            .await
            .map_err(Into::into)
    }

    async fn mt_on_transfer(
        &self,
        defuse: impl AsRef<AccountIdRef>,
        args: MtOnTransferArgs,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<U128>)> {
        let res = self
            .transaction(defuse.as_ref())
            .add_action(
                Mt::mt_on_transfer(args)
                    .gas(DEFAULT_GAS)
                    .deposit(NearToken::from_near(0)),
            )
            .wait_until(Final)
            .await?;
        let amounts = res.json::<Vec<U128>>()?;
        Ok((res.try_into()?, amounts))
    }
}
