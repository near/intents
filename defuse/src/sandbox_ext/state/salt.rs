use defuse_core::Salt;
use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountId, NearToken, serde_json::json};

pub trait SaltManagerExt {
    async fn update_current_salt(
        &self,
        defuse_contract_id: impl Into<AccountId>,
    ) -> anyhow::Result<Salt>;

    async fn invalidate_salts(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        salts: impl IntoIterator<Item = Salt>,
    ) -> anyhow::Result<Salt>;
}

pub trait SaltViewExt {
    async fn is_valid_salt(&self, salt: &Salt) -> anyhow::Result<bool>;

    async fn current_salt(&self) -> anyhow::Result<Salt>;
}

impl SaltManagerExt for SigningAccount {
    async fn update_current_salt(
        &self,
        defuse_contract_id: impl Into<AccountId>,
    ) -> anyhow::Result<Salt> {
        self.tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("update_current_salt")
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn invalidate_salts(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        salts: impl IntoIterator<Item = Salt>,
    ) -> anyhow::Result<Salt> {
        self.tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("invalidate_salts")
                    .json_args(json!({ "salts": salts.into_iter().collect::<Vec<_>>() }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
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
