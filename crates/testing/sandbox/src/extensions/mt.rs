use anyhow::Result;
use core::str;
use defuse_nep245::{Token, TokenId};
use near_kit::{AccountId, AccountIdRef, Gas, NearToken, SigningNear};
use near_sdk::json_types::U128;
use std::{
    fmt::Display,
    ops::{Bound, RangeBounds},
};

use crate::{extensions::FnCallTransaction, outcome::SuccessfulExecutionOutcome};

/// NEP-245 bindings, see [`defuse_nep245`]
#[near_kit::contract]
pub trait Mt {
    fn mt_token(&self, token_ids: &[TokenId]) -> Vec<Option<Token>>;
    fn mt_balance_of(&self, account_id: &AccountIdRef, token_id: &str) -> U128;
    fn mt_batch_balance_of(&self, account_id: &AccountIdRef, token_ids: &[TokenId]) -> Vec<U128>;
    fn mt_supply(&self, token_id: &str) -> Option<U128>;
    fn mt_batch_supply(&self, token_ids: &[TokenId]) -> Vec<Option<U128>>;

    fn mt_tokens(&self, from_index: Option<U128>, limit: Option<u32>) -> Vec<Token>;
    fn mt_tokens_for_owner(
        &self,
        account_id: &AccountIdRef,
        from_index: Option<U128>,
        limit: Option<u32>,
    ) -> Vec<Token>;

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn mt_transfer(
        &mut self,
        receiver_id: &AccountIdRef,
        token_id: &str,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<&str>,
    );

    #[call(deposit = NearToken::from_yoctonear(1))]
    fn mt_batch_transfer(
        &mut self,
        receiver_id: &AccountIdRef,
        token_ids: &[TokenId],
        amounts: &[U128],
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<&str>,
    );

    /// Returns `Vec<U128>`
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn mt_transfer_call(
        &mut self,
        receiver_id: &AccountIdRef,
        token_id: &str,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<&str>,
        msg: &str,
    );

    /// Returns `Vec<U128>`
    #[call(deposit = NearToken::from_yoctonear(1))]
    fn mt_batch_transfer_call(
        &mut self,
        receiver_id: &AccountIdRef,
        token_ids: &[TokenId],
        amounts: &[U128],
        approvals: Option<Vec<Option<(AccountId, u64)>>>,
        memo: Option<&str>,
        msg: &str,
    );

    /// Returns `Vec<U128>`
    #[call]
    fn mt_on_transfer(
        &mut self,
        sender_id: &AccountIdRef,
        previous_owner_ids: &[AccountId],
        token_ids: &[TokenId],
        amounts: &[U128],
        msg: &str,
    );
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
        sender_id: impl AsRef<AccountIdRef>,
        previous_owner_ids: impl IntoIterator<Item = AccountId>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        msg: impl AsRef<str>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<U128>)>;
}

fn range_to_pagination(range: impl RangeBounds<usize>) -> (Option<U128>, Option<u32>) {
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
    (
        from.map(|v| U128(v.try_into().unwrap())),
        limit.map(|v| v.try_into().unwrap()),
    )
}

impl MtExt for SigningNear {
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
            Mt::mt_transfer(
                receiver_id.as_ref(),
                token_id.into().as_str(),
                U128(amount),
                None,
                memo.into().as_deref(),
            )
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
                Mt::mt_batch_transfer(
                    receiver_id.as_ref(),
                    &token_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
                    &amounts.into_iter().map(U128).collect::<Vec<_>>(),
                    None,
                    memo.into().as_deref(),
                )
                .gas(Gas::from_tgas(300)),
            )
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
        let (outcome, res): (_, Vec<U128>) = self
            .fn_call_json(
                contract,
                Mt::mt_transfer_call(
                    receiver_id.as_ref(),
                    token_id.into().as_str(),
                    U128(amount),
                    None,
                    memo.into().as_deref(),
                    &msg.to_string(),
                )
                .gas(Gas::from_tgas(300)),
            )
            .await?;
        Ok((outcome, res.into_iter().map(|n| n.0).collect()))
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
        let (outcome, res): (_, Vec<U128>) = self
            .fn_call_json(
                contract,
                Mt::mt_batch_transfer_call(
                    receiver_id.as_ref(),
                    &token_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
                    &amounts.into_iter().map(U128).collect::<Vec<_>>(),
                    None,
                    memo.into().as_deref(),
                    &msg.to_string(),
                )
                .gas(Gas::from_tgas(300)),
            )
            .await?;
        Ok((outcome, res.into_iter().map(|n| n.0).collect()))
    }

    async fn mt_tokens(
        &self,
        contract: impl Into<AccountId>,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>> {
        let (from_index, limit) = range_to_pagination(range);
        self.contract::<Mt>(contract.into())
            .mt_tokens(from_index, limit)
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
            .mt_tokens_for_owner(account_id.as_ref(), from_index, limit)
            .await
            .map_err(Into::into)
    }

    async fn mt_on_transfer(
        &self,
        contract: impl Into<AccountId>,
        sender_id: impl AsRef<AccountIdRef>,
        previous_owner_ids: impl IntoIterator<Item = AccountId>,
        token_ids: impl IntoIterator<Item = impl Into<TokenId>>,
        amounts: impl IntoIterator<Item = u128>,
        msg: impl AsRef<str>,
    ) -> Result<(SuccessfulExecutionOutcome, Vec<U128>)> {
        self.fn_call_json(
            contract,
            Mt::mt_on_transfer(
                sender_id.as_ref(),
                &previous_owner_ids.into_iter().collect::<Vec<_>>(),
                &token_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
                &amounts.into_iter().map(U128).collect::<Vec<_>>(),
                msg.as_ref(),
            )
            .gas(Gas::from_tgas(300)),
        )
        .await
    }
}
