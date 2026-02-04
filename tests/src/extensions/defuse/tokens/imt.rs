use std::collections::BTreeMap;

use defuse::core::{amounts::Amounts, intents::tokens::NotifyOnTransfer, tokens::imt::ImtTokens};
use defuse_sandbox::{
    SigningAccount, anyhow, api::types::transaction::result::ExecutionSuccess, tx::FnCallBuilder,
};
use near_sdk::{AccountIdRef, NearToken, serde_json::json};

pub trait DefuseImtMinter {
    async fn imt_mint(
        &self,
        defuse_id: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        memo: Option<String>,
        notification: Option<NotifyOnTransfer>,
    ) -> anyhow::Result<(Amounts, ExecutionSuccess)>;
}

impl DefuseImtMinter for SigningAccount {
    async fn imt_mint(
        &self,
        defuse_id: impl AsRef<AccountIdRef>,
        receiver_id: impl AsRef<AccountIdRef>,
        token_ids: impl IntoIterator<Item = (impl Into<String>, u128)>,
        memo: Option<String>,
        notification: Option<NotifyOnTransfer>,
    ) -> anyhow::Result<(Amounts, ExecutionSuccess)> {
        let token_ids: ImtTokens = Amounts::new(
            token_ids
                .into_iter()
                .map(|(token, amount)| (token.into(), amount))
                .collect::<BTreeMap<String, u128>>(),
        );

        let result = self
            .tx(defuse_id.as_ref())
            .function_call(
                FnCallBuilder::new("imt_mint")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "receiver_id": receiver_id.as_ref(),
                        "tokens": token_ids,
                        "memo": memo,
                        "notification": notification
                    })),
            )
            .await?;

        Ok((result.json()?, result))
    }
}
