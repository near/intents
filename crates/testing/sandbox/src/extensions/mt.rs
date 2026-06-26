use anyhow::Result;
use core::str;
use defuse_nep245::{Token, TokenId};
use near_kit::{AccountId, AccountIdRef, Final, Gas, Near, NearToken};
use near_sdk::json_types::U128;
use serde::Serialize;
use serde_with::{DisplayFromStr, serde_as};
use std::{
    fmt::Display,
    ops::{Bound, RangeBounds},
};

use crate::{extensions::FnCallTransaction, outcome::SuccessfulExecutionOutcome};

#[derive(Serialize)]
pub struct MtTokenArgs<'a> {
    pub token_ids: &'a [TokenId],
}

#[derive(Serialize)]
pub struct MtBalanceOfArgs<'a> {
    pub account_id: &'a AccountIdRef,
    pub token_id: &'a str,
}

#[derive(Serialize)]
pub struct MtBatchBalanceOfArgs<'a> {
    pub account_id: &'a AccountIdRef,
    pub token_ids: &'a [TokenId],
}

#[derive(Serialize)]
pub struct MtSupplyArgs<'a> {
    pub token_id: &'a str,
}

#[derive(Serialize)]
pub struct MtBatchSupplyArgs<'a> {
    pub token_ids: &'a [TokenId],
}

#[serde_as]
#[derive(Serialize)]
pub struct MtTransferArgs<'a> {
    pub receiver_id: &'a AccountIdRef,
    pub token_id: &'a str,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,
    pub approval: Option<(AccountId, u64)>,
    pub memo: Option<String>,
}

#[serde_as]
#[derive(Serialize)]
pub struct MtBatchTransferArgs<'a> {
    pub receiver_id: &'a AccountIdRef,
    pub token_ids: &'a [TokenId],
    #[serde_as(as = "&[DisplayFromStr]")]
    pub amounts: &'a [u128],
    pub approvals: Option<Vec<Option<(AccountId, u64)>>>,
    pub memo: Option<String>,
}

#[serde_as]
#[derive(Serialize)]
pub struct MtTransferCallArgs<'a> {
    pub receiver_id: &'a AccountIdRef,
    pub token_id: &'a str,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u128,
    pub approval: Option<(AccountId, u64)>,
    pub memo: Option<String>,
    pub msg: &'a str,
}

#[serde_as]
#[derive(Serialize)]
pub struct MtBatchTransferCallArgs<'a> {
    pub receiver_id: &'a AccountIdRef,
    pub token_ids: &'a [TokenId],
    #[serde_as(as = "&[DisplayFromStr]")]
    pub amounts: &'a [u128],
    pub approvals: Option<Vec<Option<(AccountId, u64)>>>,
    pub memo: Option<String>,
    pub msg: &'a str,
}

#[serde_as]
#[derive(Serialize)]
pub struct MtTokensArgs {
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub from_index: Option<u128>,
    pub limit: Option<usize>,
}

#[serde_as]
#[derive(Serialize)]
pub struct MtTokensForOwnerArgs<'a> {
    pub account_id: &'a AccountIdRef,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub from_index: Option<u128>,
    pub limit: Option<usize>,
}

#[serde_as]
#[derive(Serialize)]
pub struct MtOnTransferArgs<'a> {
    pub sender_id: &'a AccountIdRef,
    pub previous_owner_ids: &'a [AccountId],
    pub token_ids: &'a [TokenId],
    #[serde_as(as = "&[DisplayFromStr]")]
    pub amounts: &'a [u128],
    pub msg: &'a str,
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
        contract: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl Into<TokenId>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn mt_batch_transfer(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn mt_transfer_call(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl Into<TokenId>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl Display,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)>;

    async fn mt_batch_transfer_call(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        memo: impl Into<Option<String>>,
        msg: impl Display,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)>;

    async fn mt_tokens(
        &self,
        contract: impl Into<AccountId>,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>>;

    async fn mt_tokens_for_owner(
        &self,
        contract: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>>;

    async fn mt_on_transfer(
        &self,
        contract: impl Into<AccountId>,
        args: MtOnTransferArgs,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<U128>)>;
}

fn range_to_pagination(range: impl RangeBounds<usize>) -> (Option<u128>, Option<usize>) {
    let from = match range.start_bound() {
        Bound::Included(v) => Some(*v),
        Bound::Excluded(v) => Some(*v + 1),
        Bound::Unbounded => None,
    };
    let to = match range.end_bound() {
        Bound::Included(v) => Some(*v + 1),
        Bound::Excluded(v) => Some(*v),
        Bound::Unbounded => None,
    };
    let limit = match (from, to) {
        (_, None) => None,
        (None, Some(v)) => Some(v),
        (Some(f), Some(t)) => Some(t - f),
    };
    (from.map(|v| v.try_into().unwrap()), limit)
}

impl MtExt for Near {
    async fn mt_transfer(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl Into<TokenId>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.fn_call(
            contract,
            Mt::mt_transfer(MtTransferArgs {
                receiver_id: receiver_id.as_ref(),
                token_id: token_id.into().as_ref(),
                amount,
                approval: None,
                memo: memo.into(),
            })
            .deposit(NearToken::from_yoctonear(1))
            .gas(Gas::from_tgas(30)),
        )
        .await
    }

    async fn mt_batch_transfer(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        memo: impl Into<Option<String>>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(contract.into())
            .add_action(
                Mt::mt_batch_transfer(MtBatchTransferArgs {
                    receiver_id: receiver_id.as_ref(),
                    token_ids: &token_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
                    amounts: &amounts.into_iter().collect::<Vec<_>>(),
                    approvals: None,
                    memo: memo.into(),
                })
                .deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(300)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn mt_transfer_call(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl Into<TokenId>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl Display,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)> {
        let outcome = self
            .transaction(contract.into())
            .add_action(
                Mt::mt_transfer_call(MtTransferCallArgs {
                    receiver_id: receiver_id.as_ref(),
                    token_id: token_id.into().as_ref(),
                    amount,
                    approval: None,
                    memo: memo.into(),
                    msg: &msg.to_string(),
                })
                .deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(300)),
            )
            .wait_until(Final)
            .await?;

        let res = outcome.json::<Vec<U128>>()?;

        Ok((outcome.try_into()?, res.into_iter().map(|n| n.0).collect()))
    }

    async fn mt_batch_transfer_call(
        &self,
        contract: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        memo: impl Into<Option<String>>,
        msg: impl Display,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<u128>)> {
        let outcome = self
            .transaction(contract.into())
            .add_action(
                Mt::mt_batch_transfer_call(MtBatchTransferCallArgs {
                    receiver_id: receiver_id.as_ref(),
                    token_ids: &token_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
                    amounts: &amounts.into_iter().collect::<Vec<_>>(),
                    approvals: None,
                    memo: memo.into(),
                    msg: &msg.to_string(),
                })
                .deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(300)),
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
        let (from_index, limit) = range_to_pagination(range);
        self.contract::<Mt>(contract.into())
            .mt_tokens(MtTokensArgs { from_index, limit })
            .await
            .map_err(Into::into)
    }

    async fn mt_tokens_for_owner(
        &self,
        contract: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>> {
        let (from_index, limit) = range_to_pagination(range);
        self.contract::<Mt>(contract.into())
            .mt_tokens_for_owner(MtTokensForOwnerArgs {
                account_id: account_id.as_ref(),
                from_index,
                limit,
            })
            .await
            .map_err(Into::into)
    }

    async fn mt_on_transfer(
        &self,
        contract: impl Into<AccountId>,
        args: MtOnTransferArgs<'_>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<U128>)> {
        let res = self
            .transaction(contract.into())
            .add_action(
                Mt::mt_on_transfer(args)
                    .deposit(NearToken::from_near(0))
                    .gas(Gas::from_tgas(300)),
            )
            .wait_until(Final)
            .await?;
        let amounts = res.json::<Vec<U128>>()?;
        Ok((res.try_into()?, amounts))
    }
}
