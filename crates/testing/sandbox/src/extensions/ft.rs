use near_sdk::{AccountId, AccountIdRef, Gas, NearToken, json_types::U128, serde_json::json};

use crate::{Account, SigningAccount, tx::FnCallBuilder};

pub trait FtExt {
    async fn ft_transfer(
        &self,
        token_id: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> anyhow::Result<()>;

    async fn ft_transfer_call(
        &self,
        token_id: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<u128>;

    async fn ft_on_transfer(
        &self,
        sender_id: impl AsRef<AccountIdRef>,
        receiver_id: impl Into<AccountId>,
        amount: u128,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<u128>;
}

pub trait FtViewExt {
    async fn ft_balance_of(&self, account_id: impl AsRef<AccountIdRef>) -> anyhow::Result<u128>;
}

impl FtExt for SigningAccount {
    async fn ft_transfer(
        &self,
        token_id: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: impl Into<Option<String>>,
    ) -> anyhow::Result<()> {
        self.tx(token_id)
            .function_call(
                FnCallBuilder::new("ft_transfer")
                    .json_args(json!({
                        "receiver_id": receiver_id.as_ref(),
                        "amount": U128(amount),
                        "memo": memo.into(),
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(())
    }

    async fn ft_transfer_call(
        &self,
        token_id: impl Into<AccountId>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: impl Into<Option<String>>,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<u128> {
        self.tx(token_id)
            .function_call(
                FnCallBuilder::new("ft_transfer_call")
                    .json_args(json!({
                        "receiver_id": receiver_id.as_ref(),
                        "amount": U128(amount),
                        "memo": memo.into(),
                        "msg": msg.as_ref(),
                    }))
                    .with_gas(Gas::from_tgas(300))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }

    async fn ft_on_transfer(
        &self,
        sender_id: impl AsRef<AccountIdRef>,
        receiver_id: impl Into<AccountId>,
        amount: u128,
        msg: impl AsRef<str>,
    ) -> anyhow::Result<u128> {
        self.tx(receiver_id)
            .function_call(FnCallBuilder::new("ft_on_transfer").json_args(json!({
                "sender_id": sender_id.as_ref(),
                "amount": U128(amount),
                "msg": msg.as_ref(),
            })))
            .await?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }
}

impl FtViewExt for Account {
    async fn ft_balance_of(&self, account_id: impl AsRef<AccountIdRef>) -> anyhow::Result<u128> {
        self.call_view_function_json::<U128>(
            "ft_balance_of",
            json!({
                "account_id": account_id.as_ref(),
            }),
        )
        .await
        .map(|v| v.0)
    }
}
