use near_api::types::errors::{DataConversionError, ExecutionError};
use near_sdk::{AccountId, AccountIdRef, Gas, NearToken, json_types::U128, serde_json::json};

use crate::{Account, SigningAccount, TxResult};

pub trait MtViewExt {
    async fn mt_batch_balance_of(
        &self,
        account_id: &AccountIdRef,
        token_ids: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Vec<u128>>;
}

pub trait MtExt {
    async fn mt_transfer(
        &self,
        contract: AccountId,
        receiver_id: &AccountIdRef,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> TxResult<()>;

    async fn mt_transfer_call(
        &self,
        contract: AccountId,
        receiver_id: &AccountIdRef,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> TxResult<u128>;

    async fn mt_on_transfer(
        &self,
        sender_id: &AccountIdRef,
        receiver_id: AccountId,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl AsRef<str>,
    ) -> TxResult<Vec<u128>>;
}

impl MtExt for SigningAccount {
    async fn mt_transfer(
        &self,
        contract: AccountId,
        receiver_id: &AccountIdRef,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> TxResult<()> {
        self.tx(contract)
            .function_call_json(
                "mt_transfer_call",
                json!({
                    "receiver_id": receiver_id,
                    "token_id": token_id.as_ref(),
                    "amount": U128(amount),
                    "memo": memo.into(),
                }),
                Gas::from_tgas(15),
                NearToken::from_yoctonear(1),
            )
            .await
    }

    async fn mt_transfer_call(
        &self,
        contract: AccountId,
        receiver_id: &AccountIdRef,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> TxResult<u128> {
        self.tx(contract)
            .function_call_json::<Vec<U128>>(
                "mt_transfer_call",
                json!({
                    "receiver_id": receiver_id,
                    "token_id": token_id.as_ref(),
                    "amount": U128(amount),
                    "memo": memo.into(),
                    "msg": msg.into(),
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(1),
            )
            .await
            .and_then(|amounts| {
                let [amount] = amounts
                    .try_into()
                    .map_err(|amounts: Vec<_>| DataConversionError::IncorrectLength(amounts.len()))
                    .map_err(Into::<ExecutionError>::into)?;
                Ok(amount.0)
            })
    }

    async fn mt_on_transfer(
        &self,
        sender_id: &AccountIdRef,
        receiver_id: AccountId,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl AsRef<str>,
    ) -> TxResult<Vec<u128>> {
        let (token_ids, amounts): (Vec<_>, Vec<_>) = token_ids
            .into_iter()
            .map(|(token_id, amount)| (token_id.into(), U128(amount)))
            .unzip();

        self.tx(receiver_id)
            .function_call_json::<Vec<U128>>(
                "mt_on_transfer",
                json!({
                    "sender_id": sender_id,
                    "previous_owner_ids": [sender_id],
                    "token_ids": token_ids,
                    "amounts": amounts,
                    "msg": msg.as_ref(),
                }),
                Gas::from_tgas(250),
                NearToken::from_yoctonear(0),
            )
            .await
            .map(|refunds| refunds.into_iter().map(|a| a.0).collect())
    }
}

impl MtViewExt for Account {
    async fn mt_batch_balance_of(
        &self,
        account_id: &AccountIdRef,
        token_ids: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Vec<u128>> {
        self.call_view_function_json::<Vec<U128>>(
            "mt_batch_balance_of",
            json!({
                "account_id": account_id,
                "token_ids": token_ids.into_iter().collect::<Vec<_>>()
            }),
        )
        .await
        .map(|balances| balances.into_iter().map(|b| b.0).collect())
    }
}

impl MtViewExt for SigningAccount {
    async fn mt_batch_balance_of(
        &self,
        account_id: &AccountIdRef,
        token_ids: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Vec<u128>> {
        self.account()
            .mt_batch_balance_of(account_id, token_ids)
            .await
    }
}
