use std::collections::BTreeMap;

use crate::{SigningAccount, anyhow, tx::FnCallBuilder};
use defuse::core::{amounts::Amounts, tokens::imt::ImtTokens};
use near_sdk::{AccountIdRef, NearToken, serde_json::json};

pub trait DefuseImtBurner {
    async fn imt_burn(
        &self,
        defuse_id: impl AsRef<AccountIdRef>,
        minter_id: impl AsRef<AccountIdRef>,
        tokens: impl IntoIterator<Item = (impl Into<String>, u128)>,
        memo: impl Into<Option<String>>,
    ) -> anyhow::Result<()>;
}

impl DefuseImtBurner for SigningAccount {
    async fn imt_burn(
        &self,
        defuse_id: impl AsRef<AccountIdRef>,
        minter_id: impl AsRef<AccountIdRef>,
        tokens: impl IntoIterator<Item = (impl Into<String>, u128)>,
        memo: impl Into<Option<String>>,
    ) -> anyhow::Result<()> {
        let token_ids: ImtTokens = Amounts::new(
            tokens
                .into_iter()
                .map(|(token, amount)| (token.into(), amount))
                .collect::<BTreeMap<String, u128>>(),
        );

        self.tx(defuse_id.as_ref())
            .function_call(
                FnCallBuilder::new("imt_burn")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "minter_id": minter_id.as_ref(),
                        "tokens": token_ids,
                        "memo": memo.into(),
                    })),
            )
            .await?;

        Ok(())
    }
}
