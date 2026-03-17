use std::ops::RangeBounds;

use defuse_nep245::Token;
use near_api::types::transaction::result::ExecutionFinalResult;
use near_sdk::{AccountId, AccountIdRef, Gas, NearToken, json_types::U128, serde_json::json};

use crate::{Account, SigningAccount, tx::FnCallBuilder};

pub trait MtExt {
    async fn mt_transfer(
        &self,
        contract_id: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> anyhow::Result<()>;

    async fn mt_transfer_call(
        &self,
        contract_id: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<u128>;

    async fn mt_batch_transfer_call(
        &self,
        contract_id: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = String>,
        amounts: impl IntoIterator<Item = u128>,
        memo: impl Into<Option<String>>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<ExecutionFinalResult>;

    async fn mt_on_transfer(
        &self,
        sender_id: impl AsRef<AccountIdRef>,
        receiver_id: impl Into<AccountId>,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<Vec<u128>>;

    /// Same as `mt_on_transfer` but returns the full execution result for receipt inspection
    async fn mt_on_transfer_raw(
        &self,
        sender_id: impl AsRef<AccountIdRef>,
        receiver_id: impl Into<AccountId>,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<ExecutionFinalResult>;
}

pub trait MtViewExt {
    async fn mt_batch_balance_of(
        &self,
        account_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Vec<u128>>;

    async fn mt_balance_of(
        &self,
        account_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
    ) -> anyhow::Result<u128>;

    async fn mt_tokens(&self, range: impl RangeBounds<usize>) -> anyhow::Result<Vec<Token>>;

    async fn mt_tokens_for_owner(
        &self,
        account_id: impl AsRef<AccountIdRef>,
        range: impl RangeBounds<usize>,
    ) -> anyhow::Result<Vec<Token>>;
}

impl MtExt for SigningAccount {
    async fn mt_transfer(
        &self,
        contract_id: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> anyhow::Result<()> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("mt_transfer")
                    .json_args(json!({
                        "receiver_id": receiver_id.as_ref(),
                        "token_id": token_id.as_ref(),
                        "amount": U128(amount),
                        "memo": memo.into(),
                    }))
                    .with_gas(Gas::from_tgas(15))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(())
    }

    async fn mt_transfer_call(
        &self,
        contract_id: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<u128> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("mt_transfer_call")
                    .json_args(json!({
                        "receiver_id": receiver_id.as_ref(),
                        "token_id": token_id.as_ref(),
                        "amount": U128(amount),
                        "memo": memo.into(),
                        "msg": msg.as_ref(),
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json::<[U128; 1]>()
            .map(|v| v[0].0)
            .map_err(Into::into)
    }

    async fn mt_batch_transfer_call(
        &self,
        contract_id: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = String>,
        amounts: impl IntoIterator<Item = u128>,
        memo: impl Into<Option<String>>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<ExecutionFinalResult> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("mt_batch_transfer_call")
                    .json_args(json!({
                        "receiver_id": receiver_id.as_ref(),
                        "token_ids": token_ids.into_iter().collect::<Vec<_>>(),
                        "amounts": amounts.into_iter().map(U128::from).collect::<Vec<_>>(),
                        "approvals": Option::<Vec<Option<(near_sdk::AccountId, u64)>>>::None,
                        "memo": memo.into(),
                        "msg": msg.as_ref(),
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .exec_transaction()
            .await
    }

    async fn mt_on_transfer(
        &self,
        sender_id: impl AsRef<AccountIdRef>,
        receiver_id: impl Into<AccountId>,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<Vec<u128>> {
        self.mt_on_transfer_raw(sender_id, receiver_id, token_ids, msg)
            .await?
            .into_result()?
            .json::<Vec<U128>>()
            .map(|refunds| refunds.into_iter().map(|a| a.0).collect())
            .map_err(Into::into)
    }

    async fn mt_on_transfer_raw(
        &self,
        sender_id: impl AsRef<AccountIdRef>,
        receiver_id: impl Into<AccountId>,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<ExecutionFinalResult> {
        let (token_ids, amounts): (Vec<_>, Vec<_>) = token_ids
            .into_iter()
            .map(|(token_id, amount)| (token_id.into(), U128(amount)))
            .unzip();

        self.tx(receiver_id)
            .function_call(FnCallBuilder::new("mt_on_transfer").json_args(json!({
                "sender_id": sender_id.as_ref(),
                "previous_owner_ids": vec![sender_id.as_ref(); token_ids.len()],
                "token_ids": token_ids,
                "amounts": amounts,
                "msg": msg.as_ref(),
            })))
            .exec_transaction()
            .await
    }
}

impl MtViewExt for Account {
    async fn mt_batch_balance_of(
        &self,
        account_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Vec<u128>> {
        self.call_view_function_json::<Vec<U128>>(
            "mt_batch_balance_of",
            json!({
                "account_id": account_id.as_ref(),
                "token_ids": token_ids.into_iter().collect::<Vec<_>>()
            }),
        )
        .await
        .map(|balances| balances.into_iter().map(|b| b.0).collect())
    }

    async fn mt_balance_of(
        &self,
        account_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<str>,
    ) -> anyhow::Result<u128> {
        self.call_view_function_json::<U128>(
            "mt_balance_of",
            json!({
                "account_id": account_id.as_ref(),
                "token_id": token_id.as_ref(),
            }),
        )
        .await
        .map(|b| b.0)
    }

    async fn mt_tokens(&self, range: impl RangeBounds<usize>) -> anyhow::Result<Vec<Token>> {
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

        self.call_view_function_json(
            "mt_tokens",
            json!({
                "from_index": from.map(|v| U128(v.try_into().unwrap())),
                "limit": limit,
            }),
        )
        .await
    }

    async fn mt_tokens_for_owner(
        &self,
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

        self.call_view_function_json(
            "mt_tokens_for_owner",
            json!({
                "account_id": account_id.as_ref(),
                "from_index": from.map(|v| U128(v.try_into().unwrap())),
                "limit": limit,
            }),
        )
        .await
    }
}
