use near_sdk::{AccountIdRef, Gas, NearToken, json_types::U128, serde_json::json};

use crate::{Account, SigningAccount, tx::FnCallBuilder};

#[allow(async_fn_in_trait)]
pub trait FtExt {
    async fn ft_transfer(
        &self,
        token_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
    ) -> anyhow::Result<()>;

    async fn ft_transfer_call(
        &self,
        token_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
        msg: &str,
    ) -> anyhow::Result<u128>;
}

#[allow(async_fn_in_trait)]
pub trait FtViewExt {
    async fn ft_balance_of(&self, account_id: &AccountIdRef) -> anyhow::Result<u128>;
}

impl FtExt for SigningAccount {
    async fn ft_transfer(
        &self,
        token_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.tx(token_id.into())
            .function_call(
                FnCallBuilder::new("ft_transfer")
                    .json_args(&json!({
                        "receiver_id": receiver_id,
                        "amount": U128(amount),
                        "memo": memo,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(())
    }

    async fn ft_transfer_call(
        &self,
        token_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
        msg: &str,
    ) -> anyhow::Result<u128> {
        self.tx(token_id.into())
            .function_call(
                FnCallBuilder::new("ft_transfer_call")
                    .json_args(&json!({
                        "receiver_id": receiver_id,
                        "amount": U128(amount),
                        "memo": memo,
                        "msg": msg,
                    }))
                    .with_gas(Gas::from_tgas(300))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }
}

impl FtViewExt for Account {
    async fn ft_balance_of(&self, account_id: &AccountIdRef) -> anyhow::Result<u128> {
        self.call_view_function_json::<U128>(
            "ft_balance_of",
            json!({
                "account_id": account_id,
            }),
        )
        .await
        .map(|v| v.0)
    }
}

impl FtViewExt for SigningAccount {
    async fn ft_balance_of(&self, account_id: &AccountIdRef) -> anyhow::Result<u128> {
        self.account().ft_balance_of(account_id).await
    }
}
