use defuse_nep245::TokenId;
use defuse_sandbox::{
    SigningAccount, anyhow,
    api::types::transaction::result::{ExecutionResult, Value},
    extensions::mt::MtExt,
    tx::FnCallBuilder,
};
use near_sdk::{AccountIdRef, NearToken, json_types::U128, serde_json::json};

pub trait DefuseMtDepositer {
    async fn defuse_mt_deposit(
        &self,
        sender_id: &AccountIdRef,
        defuse_id: &AccountIdRef,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl Into<Option<String>>,
    ) -> anyhow::Result<Vec<u128>>;
}

pub trait DefuseMtWithdrawer {
    async fn defuse_mt_withdraw(
        &self,
        defuse_id: &AccountIdRef,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_ids: Vec<TokenId>,
        amounts: Vec<u128>,
        msg: Option<String>,
    ) -> anyhow::Result<(Vec<u128>, ExecutionResult<Value>)>;
}

impl DefuseMtDepositer for SigningAccount {
    async fn defuse_mt_deposit(
        &self,
        sender_id: &AccountIdRef,
        defuse_id: &AccountIdRef,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl Into<Option<String>>,
    ) -> anyhow::Result<Vec<u128>> {
        self.mt_on_transfer(
            sender_id.into(),
            defuse_id.into(),
            token_ids,
            &msg.into()
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
        .await
        .map_err(Into::into)
    }
}

impl DefuseMtWithdrawer for SigningAccount {
    async fn defuse_mt_withdraw(
        &self,
        defuse_id: &AccountIdRef,
        token: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_ids: Vec<TokenId>,
        amounts: Vec<u128>,
        msg: Option<String>,
    ) -> anyhow::Result<(Vec<u128>, ExecutionResult<Value>)> {
        let res = self
            .tx(defuse_id.into())
            .function_call(
                FnCallBuilder::new("mt_withdraw")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(&json!({
                        "token": token,
                        "receiver_id": receiver_id,
                        "token_ids": token_ids,
                        "amounts": amounts.into_iter().map(U128).collect::<Vec<_>>(),
                        "msg": msg
                    })),
            )
            .await?;

        let values = res
            .json::<Vec<U128>>()
            .map(|v| v.into_iter().map(|val| val.0).collect())?;

        Ok((values, res))
    }
}
