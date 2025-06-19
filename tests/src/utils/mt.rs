use std::ops::RangeBounds;

use crate::utils::test_log::TestLog;
use defuse::nep245::{Token, TokenId};
use near_sdk::{AccountId, AccountIdRef, NearToken, json_types::U128};
use serde_json::json;

pub trait MtExt {
    async fn mt_transfer(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: u128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    ) -> anyhow::Result<()>;

    #[allow(clippy::too_many_arguments)]
    async fn mt_transfer_call(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: u128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<()>;

    async fn mt_batch_transfer(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_ids: impl IntoIterator<Item = TokenId>,
        amounts: impl IntoIterator<Item = u128>,
        approvals: Option<impl IntoIterator<Item = Option<(AccountId, u64)>>>,
        memo: Option<String>,
    ) -> anyhow::Result<()>;

    #[allow(clippy::too_many_arguments)]
    async fn mt_batch_transfer_call(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_ids: impl IntoIterator<Item = TokenId>,
        amounts: impl IntoIterator<Item = u128>,
        approvals: Option<impl IntoIterator<Item = Option<(AccountId, u64)>>>,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<(Vec<u128>, TestLog)>;

    async fn mt_contract_balance_of(
        &self,
        token_contract: &AccountId,
        account_id: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<u128>;

    async fn mt_balance_of(
        &self,
        account_id: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<u128>;

    async fn mt_contract_batch_balance_of(
        &self,
        token_contract: &AccountId,
        account_id: &AccountId,
        token_ids: impl IntoIterator<Item = &TokenId>,
    ) -> anyhow::Result<Vec<u128>>;

    async fn mt_batch_balance_of(
        &self,
        account_id: &AccountId,
        token_ids: impl IntoIterator<Item = &TokenId>,
    ) -> anyhow::Result<Vec<u128>>;

    async fn mt_tokens(
        &self,
        token_contract: &AccountId,
        from_index: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>>;

    async fn mt_tokens_for_owner(
        &self,
        token_contract: &AccountId,
        account_id: &AccountIdRef,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>>;
}

impl MtExt for near_workspaces::Account {
    async fn mt_transfer(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: u128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.call(token_contract, "mt_transfer")
            .args_json(json!({
                "receiver_id": receiver_id,
                "token_id": token_id,
                "amount": U128(amount),
                "approval": approval,
                "memo": memo,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn mt_transfer_call(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: u128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<()> {
        self.call(token_contract, "mt_transfer_call")
            .args_json(json!({
                "receiver_id": receiver_id,
                "token_id": token_id,
                "amount": U128(amount),
                "approval": approval,
                "memo": memo,
                "msg": msg
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn mt_batch_transfer(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_ids: impl IntoIterator<Item = TokenId>,
        amounts: impl IntoIterator<Item = u128>,
        approvals: Option<impl IntoIterator<Item = Option<(AccountId, u64)>>>,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.call(token_contract, "mt_batch_transfer")
            .args_json(json!({
                "receiver_id": receiver_id,
                "token_ids": token_ids.into_iter().collect::<Vec<_>>(),
                "amounts": amounts.into_iter().collect::<Vec<_>>(),
                "approvals": approvals.map(|a| a.into_iter().collect::<Vec<_>>()),
                "memo": memo,

            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn mt_batch_transfer_call(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_ids: impl IntoIterator<Item = TokenId>,
        amounts: impl IntoIterator<Item = u128>,
        approvals: Option<impl IntoIterator<Item = Option<(AccountId, u64)>>>,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<(Vec<u128>, TestLog)> {
        let result = self
            .call(token_contract, "mt_batch_transfer_call")
            .args_json(json!({
                "receiver_id": receiver_id,
                "token_ids": token_ids.into_iter().collect::<Vec<_>>(),
                "amounts": amounts.into_iter().map(U128).collect::<Vec<_>>(),
                "approvals": approvals.map(|a| a.into_iter().collect::<Vec<_>>()),
                "memo": memo,
                "msg": msg
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok((
            result
                .clone()
                .json::<Vec<U128>>()?
                .into_iter()
                .map(|a| a.0)
                .collect(),
            result.into(),
        ))
    }

    async fn mt_contract_balance_of(
        &self,
        token_contract: &AccountId,
        account_id: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<u128> {
        self.view(token_contract, "mt_balance_of")
            .args_json(json!({
                "account_id": account_id,
                "token_id": token_id,
            }))
            .await?
            .json::<U128>()
            .map(|b| b.0)
            .map_err(Into::into)
    }

    async fn mt_balance_of(
        &self,
        account_id: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<u128> {
        self.mt_contract_balance_of(self.id(), account_id, token_id)
            .await
    }

    async fn mt_contract_batch_balance_of(
        &self,
        token_contract: &AccountId,
        account_id: &AccountId,
        token_ids: impl IntoIterator<Item = &TokenId>,
    ) -> anyhow::Result<Vec<u128>> {
        self.view(token_contract, "mt_batch_balance_of")
            .args_json(json!({
                "account_id": account_id,
                "token_ids": token_ids.into_iter().collect::<Vec<_>>(),
            }))
            .await?
            .json::<Vec<U128>>()
            .map(|bs| bs.into_iter().map(|bs| bs.0).collect())
            .map_err(Into::into)
    }

    async fn mt_batch_balance_of(
        &self,
        account_id: &AccountId,
        token_ids: impl IntoIterator<Item = &TokenId>,
    ) -> anyhow::Result<Vec<u128>> {
        self.mt_contract_batch_balance_of(self.id(), account_id, token_ids)
            .await
    }

    async fn mt_tokens(
        &self,
        token_contract: &AccountId,
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

        let res = self
            .view(token_contract, "mt_tokens")
            .args_json(json!({
                "from_index": from.map(|v| U128(v.try_into().unwrap())),
                "limit": limit,
            }))
            .await?
            .json::<Vec<Token>>()?;

        Ok(res)
    }

    async fn mt_tokens_for_owner(
        &self,
        token_contract: &AccountId,
        account_id: &AccountIdRef,
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

        let res = self
            .view(token_contract, "mt_tokens_for_owner")
            .args_json(json!({
                "account_id": account_id,
                "from_index": from.map(|v| U128(v.try_into().unwrap())),
                "limit": limit,
            }))
            .await?
            .json::<Vec<Token>>()?;

        Ok(res)
    }
}

impl MtExt for near_workspaces::Contract {
    async fn mt_transfer(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: u128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.as_account()
            .mt_transfer(
                token_contract,
                receiver_id,
                token_id,
                amount,
                approval,
                memo,
            )
            .await
    }

    async fn mt_transfer_call(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        amount: u128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<()> {
        self.as_account()
            .mt_transfer_call(
                token_contract,
                receiver_id,
                token_id,
                amount,
                approval,
                memo,
                msg,
            )
            .await
    }

    async fn mt_batch_transfer(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_ids: impl IntoIterator<Item = TokenId>,
        amounts: impl IntoIterator<Item = u128>,
        approvals: Option<impl IntoIterator<Item = Option<(AccountId, u64)>>>,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.as_account()
            .mt_batch_transfer(
                token_contract,
                receiver_id,
                token_ids,
                amounts,
                approvals,
                memo,
            )
            .await
    }

    async fn mt_batch_transfer_call(
        &self,
        token_contract: &AccountId,
        receiver_id: &AccountId,
        token_ids: impl IntoIterator<Item = TokenId>,
        amounts: impl IntoIterator<Item = u128>,
        approvals: Option<impl IntoIterator<Item = Option<(AccountId, u64)>>>,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<(Vec<u128>, TestLog)> {
        self.as_account()
            .mt_batch_transfer_call(
                token_contract,
                receiver_id,
                token_ids,
                amounts,
                approvals,
                memo,
                msg,
            )
            .await
    }

    async fn mt_contract_balance_of(
        &self,
        token_contract: &AccountId,
        account_id: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<u128> {
        self.as_account()
            .mt_contract_balance_of(token_contract, account_id, token_id)
            .await
    }

    async fn mt_balance_of(
        &self,
        account_id: &AccountId,
        token_id: &TokenId,
    ) -> anyhow::Result<u128> {
        self.as_account().mt_balance_of(account_id, token_id).await
    }

    async fn mt_contract_batch_balance_of(
        &self,
        token_contract: &AccountId,
        account_id: &AccountId,
        token_ids: impl IntoIterator<Item = &TokenId>,
    ) -> anyhow::Result<Vec<u128>> {
        self.as_account()
            .mt_contract_batch_balance_of(token_contract, account_id, token_ids)
            .await
    }

    async fn mt_batch_balance_of(
        &self,
        account_id: &AccountId,
        token_ids: impl IntoIterator<Item = &TokenId>,
    ) -> anyhow::Result<Vec<u128>> {
        self.as_account()
            .mt_batch_balance_of(account_id, token_ids)
            .await
    }

    async fn mt_tokens(
        &self,
        token_contract: &AccountId,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>> {
        self.as_account().mt_tokens(token_contract, range).await
    }

    async fn mt_tokens_for_owner(
        &self,
        token_contract: &AccountId,
        account_id: &AccountIdRef,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>> {
        self.as_account()
            .mt_tokens_for_owner(token_contract, account_id, range)
            .await
    }
}
