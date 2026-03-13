use crate::{
    SigningAccount, anyhow, api::types::transaction::result::ExecutionSuccess,
    extensions::mt::MtExt, tx::FnCallBuilder,
};
use defuse_nep245::TokenId;
use near_sdk::{AccountId, AccountIdRef, NearToken, json_types::U128, serde_json::json};

pub trait DefuseMtDepositor: MtExt {
    async fn defuse_mt_deposit(
        &self,
        sender_id: impl AsRef<AccountIdRef>,
        defuse_id: impl Into<AccountId>,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        msg: impl Into<Option<String>>,
    ) -> anyhow::Result<Vec<u128>> {
        self.mt_on_transfer(
            sender_id,
            defuse_id,
            token_ids,
            &msg.into()
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
        .await
    }
}

impl<T> DefuseMtDepositor for T where T: MtExt {}

pub trait DefuseMtWithdrawer {
    async fn defuse_mt_withdraw(
        &self,
        defuse_id: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: Vec<TokenId>,
        amounts: Vec<u128>,
        msg: Option<String>,
    ) -> anyhow::Result<(Vec<u128>, ExecutionSuccess)>;
}

impl DefuseMtWithdrawer for SigningAccount {
    async fn defuse_mt_withdraw(
        &self,
        defuse_id: impl Into<AccountId>,
        token: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: Vec<TokenId>,
        amounts: Vec<u128>,
        msg: Option<String>,
    ) -> anyhow::Result<(Vec<u128>, ExecutionSuccess)> {
        let res = self
            .tx(defuse_id)
            .function_call(
                FnCallBuilder::new("mt_withdraw")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "token": token.as_ref(),
                        "receiver_id": receiver_id.as_ref(),
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
