use near_api::types::errors::DataConversionError;
use near_sdk::{AccountId, AccountIdRef, Gas, NearToken, json_types::U128, serde_json::json};

use crate::{Account, SigningAccount, tx::FnCallBuilder};

#[allow(async_fn_in_trait)]
pub trait MtExt {
    async fn mt_transfer(
        &self,
        contract: AccountId,
        receiver_id: &AccountIdRef,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> anyhow::Result<()>;

    async fn mt_transfer_call(
        &self,
        contract: AccountId,
        receiver_id: &AccountIdRef,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> anyhow::Result<u128>;

    async fn mt_on_transfer(
        &self,
        sender_id: &AccountIdRef,
        receiver_id: AccountId,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<Vec<u128>>;
}

#[allow(async_fn_in_trait)]
pub trait MtViewExt {
    async fn mt_batch_balance_of(
        &self,
        account_id: &AccountIdRef,
        token_ids: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Vec<u128>>;
}

impl MtExt for SigningAccount {
    async fn mt_transfer(
        &self,
        contract: AccountId,
        receiver_id: &AccountIdRef,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> anyhow::Result<()> {
        self.tx(contract)
            .function_call(
                FnCallBuilder::new("mt_transfer")
                    .json_args(&json!({
                        "receiver_id": receiver_id,
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
        contract: AccountId,
        receiver_id: &AccountIdRef,
        token_id: impl AsRef<str>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl Into<String>,
    ) -> anyhow::Result<u128> {
        self.tx(contract)
            .function_call(
                FnCallBuilder::new("mt_transfer_call")
                    .json_args(&json!({
                        "receiver_id": receiver_id,
                        "token_id": token_id.as_ref(),
                        "amount": U128(amount),
                        "memo": memo.into(),
                        "msg": msg.into(),
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json::<Vec<U128>>()
            .and_then(|amounts| {
                let [amount] = amounts.try_into().map_err(|amounts: Vec<_>| {
                    DataConversionError::IncorrectLength(amounts.len())
                })?;
                Ok(amount.0)
            })
            .map_err(Into::into)
    }

    async fn mt_on_transfer(
        &self,
        sender_id: &AccountIdRef,
        receiver_id: AccountId,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<Vec<u128>> {
        let (token_ids, amounts): (Vec<_>, Vec<_>) = token_ids
            .into_iter()
            .map(|(token_id, amount)| (token_id.into(), U128(amount)))
            .unzip();

        self.tx(receiver_id)
            .function_call(FnCallBuilder::new("mt_on_transfer").json_args(&json!({
                "sender_id": sender_id,
                "previous_owner_ids": [sender_id],
                "token_ids": token_ids,
                "amounts": amounts,
                "msg": msg.as_ref(),
            })))
            .await?
            .json::<Vec<U128>>()
            .map(|refunds| refunds.into_iter().map(|a| a.0).collect())
            .map_err(Into::into)
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
