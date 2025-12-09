#![allow(clippy::too_many_arguments)]

use defuse_sandbox::{SigningAccount, anyhow, extensions::ft::FtExt, tx::FnCallBuilder};
use near_sdk::{AccountIdRef, NearToken, json_types::U128, serde_json::json};

use crate::tokens::DepositMessage;

#[allow(async_fn_in_trait)]
pub trait DefuseFtReceiver {
    async fn defuse_ft_deposit(
        &self,
        defuse_id: &AccountIdRef,
        token_id: &AccountIdRef,
        amount: u128,
        msg: impl Into<Option<DepositMessage>>,
    ) -> anyhow::Result<u128>;
}

#[allow(async_fn_in_trait)]
pub trait DefuseFtWithdrawer {
    async fn defuse_ft_withdraw(
        &self,
        defuse_id: &AccountIdRef,
        token_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128>;

    async fn defuse_ft_force_withdraw(
        &self,
        defuse_id: &AccountIdRef,
        owner_id: &AccountIdRef,
        token_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128>;
}

// TODO: may be replace it with ft_on_transfer?
impl DefuseFtReceiver for SigningAccount {
    async fn defuse_ft_deposit(
        &self,
        defuse_id: &AccountIdRef,
        token_id: &AccountIdRef,
        amount: u128,
        msg: impl Into<Option<DepositMessage>>,
    ) -> anyhow::Result<u128> {
        self.ft_transfer_call(
            token_id,
            defuse_id,
            amount,
            None,
            &msg.into()
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
        .await
    }
}

impl DefuseFtWithdrawer for SigningAccount {
    async fn defuse_ft_withdraw(
        &self,
        defuse_id: &AccountIdRef,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128> {
        self.tx(defuse_id.into())
            .function_call(
                FnCallBuilder::new("ft_withdraw")
                    .json_args(json!({
                        "token": token,
                        "receiver_id": receiver_id,
                        "amount": U128(amount),
                        "memo": memo,
                        "msg": msg,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }

    async fn defuse_ft_force_withdraw(
        &self,
        defuse_id: &AccountIdRef,
        owner_id: &AccountIdRef,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128> {
        self.tx(defuse_id.into())
            .function_call(
                FnCallBuilder::new("ft_force_withdraw")
                    .json_args(json!({
                        "owner_id": owner_id,
                        "token": token,
                        "receiver_id": receiver_id,
                        "amount": U128(amount),
                        "memo": memo,
                        "msg": msg,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }
}
