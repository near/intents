use crate::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use defuse::core::Salt;
use near_api::types::transaction::result::ExecutionSuccess;
use near_sdk::{AccountId, NearToken, serde_json::json};

pub trait SaltManagerExt {
    async fn update_current_salt(
        &self,
        defuse_contract_id: impl Into<AccountId>,
    ) -> anyhow::Result<(ExecutionSuccess, Salt)>;

    async fn invalidate_salts(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        salts: impl IntoIterator<Item = Salt>,
    ) -> anyhow::Result<(ExecutionSuccess, Salt)>;
}

pub trait SaltViewExt {
    async fn is_valid_salt(&self, salt: &Salt) -> anyhow::Result<bool>;

    async fn current_salt(&self) -> anyhow::Result<Salt>;
}

impl SaltManagerExt for SigningAccount {
    async fn update_current_salt(
        &self,
        defuse_contract_id: impl Into<AccountId>,
    ) -> anyhow::Result<(ExecutionSuccess, Salt)> {
        let res = self
            .tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("update_current_salt")
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        let salt = res.json()?;

        Ok((res, salt))
    }

    async fn invalidate_salts(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        salts: impl IntoIterator<Item = Salt>,
    ) -> anyhow::Result<(ExecutionSuccess, Salt)> {
        let res = self
            .tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("invalidate_salts")
                    .json_args(json!({ "salts": salts.into_iter().collect::<Vec<_>>() }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        let salt = res.json()?;

        Ok((res, salt))
    }
}

impl SaltViewExt for Account {
    async fn is_valid_salt(&self, salt: &Salt) -> anyhow::Result<bool> {
        self.call_view_function_json("is_valid_salt", json!({ "salt": salt }))
            .await
    }

    async fn current_salt(&self) -> anyhow::Result<Salt> {
        self.call_view_function_json("current_salt", ()).await
    }
}
