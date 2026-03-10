#![allow(clippy::too_many_arguments)]

use crate::{SigningAccount, anyhow, extensions::ft::FtExt, tx::FnCallBuilder};
use near_sdk::{AccountId, AccountIdRef, NearToken, json_types::U128, serde_json::json};

use defuse::tokens::DepositMessage;

pub trait DefuseFtDepositor: FtExt {
    async fn defuse_ft_deposit(
        &self,
        defuse_id: impl AsRef<AccountIdRef>,
        token_id: impl Into<AccountId>,
        amount: u128,
        msg: impl Into<Option<DepositMessage>>,
    ) -> anyhow::Result<u128> {
        self.ft_transfer_call(
            token_id,
            defuse_id,
            amount,
            "deposit".to_string(),
            msg.into()
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
        .await
    }
}

impl<T> DefuseFtDepositor for T where T: FtExt {}

pub trait DefuseFtWithdrawer {
    async fn defuse_ft_withdraw(
        &self,
        defuse_id: impl Into<AccountId>,
        token_id: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128>;

    async fn defuse_ft_force_withdraw(
        &self,
        defuse_id: impl Into<AccountId>,
        owner_id: impl AsRef<AccountIdRef>,
        token_id: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128>;
}

impl DefuseFtWithdrawer for SigningAccount {
    async fn defuse_ft_withdraw(
        &self,
        defuse_id: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128> {
        self.tx(defuse_id)
            .function_call(
                FnCallBuilder::new("ft_withdraw")
                    .json_args(json!({
                        "token": token.as_ref(),
                        "receiver_id": receiver_id.as_ref(),
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
        defuse_id: impl Into<AccountId>,
        owner_id: impl AsRef<AccountIdRef>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128> {
        self.tx(defuse_id)
            .function_call(
                FnCallBuilder::new("ft_force_withdraw")
                    .json_args(json!({
                        "owner_id": owner_id.as_ref(),
                        "token": token.as_ref(),
                        "receiver_id": receiver_id.as_ref(),
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
