#![allow(clippy::too_many_arguments)]

use defuse::tokens::DepositMessage;
use near_sdk::{AccountId, NearToken, json_types::U128};
use serde_json::json;

use crate::utils::ft::FtExt;

pub trait DefuseFtReceiver {
    async fn defuse_ft_deposit(
        &self,
        defuse_id: &AccountId,
        token_id: &AccountId,
        amount: u128,
        msg: impl Into<Option<DepositMessage>>,
    ) -> anyhow::Result<u128>;
}

pub trait DefuseFtWithdrawer {
    async fn defuse_ft_withdraw(
        &self,
        defuse_id: &AccountId,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128>;

    async fn defuse_ft_force_withdraw(
        &self,
        defuse_id: &AccountId,
        owner_id: &AccountId,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128>;
}

impl DefuseFtReceiver for near_workspaces::Account {
    async fn defuse_ft_deposit(
        &self,
        defuse_id: &AccountId,
        token_id: &AccountId,
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

impl DefuseFtReceiver for near_workspaces::Contract {
    async fn defuse_ft_deposit(
        &self,
        defuse_id: &AccountId,
        token_id: &AccountId,
        amount: u128,
        msg: impl Into<Option<DepositMessage>>,
    ) -> anyhow::Result<u128> {
        self.as_account()
            .defuse_ft_deposit(defuse_id, token_id, amount, msg)
            .await
    }
}

impl DefuseFtWithdrawer for near_workspaces::Account {
    async fn defuse_ft_withdraw(
        &self,
        defuse_id: &AccountId,
        token: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128> {
        self.call(defuse_id, "ft_withdraw")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "token": token,
                "receiver_id": receiver_id,
                "amount": U128(amount),
                "memo": memo,
                "msg": msg,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }

    async fn defuse_ft_force_withdraw(
        &self,
        defuse_id: &AccountId,
        owner_id: &AccountId,
        token: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128> {
        self.call(defuse_id, "ft_force_withdraw")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "owner_id": owner_id,
                "token": token,
                "receiver_id": receiver_id,
                "amount": U128(amount),
                "memo": memo,
                "msg": msg,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }
}

impl DefuseFtWithdrawer for near_workspaces::Contract {
    async fn defuse_ft_withdraw(
        &self,
        defuse_id: &AccountId,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128> {
        self.as_account()
            .defuse_ft_withdraw(defuse_id, token_id, receiver_id, amount, memo, msg)
            .await
    }

    async fn defuse_ft_force_withdraw(
        &self,
        defuse_id: &AccountId,
        owner_id: &AccountId,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
        msg: Option<String>,
    ) -> anyhow::Result<u128> {
        self.as_account()
            .defuse_ft_force_withdraw(
                defuse_id,
                owner_id,
                token_id,
                receiver_id,
                amount,
                memo,
                msg,
            )
            .await
    }
}
