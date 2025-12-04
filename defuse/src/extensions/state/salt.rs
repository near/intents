use defuse_core::Salt;
use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountId, NearToken, serde_json::json};

pub trait SaltManagerExt {
    async fn update_current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt>;

    async fn invalidate_salts(
        &self,
        defuse_contract_id: &AccountId,
        salts: &[Salt],
    ) -> anyhow::Result<Salt>;
}

pub trait SaltManagerViewExt {
    async fn is_valid_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: &Salt,
    ) -> anyhow::Result<bool>;

    async fn current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt>;
}

impl SaltManagerExt for SigningAccount {
    async fn update_current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt> {
        self.tx(defuse_contract_id.clone())
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
        defuse_contract_id: &AccountId,
        salts: &[Salt],
    ) -> anyhow::Result<Salt> {
        self.tx(defuse_contract_id.clone())
            .function_call(
                FnCallBuilder::new("invalidate_salts")
                    .json_args(&json!({ "salts": salts }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl SaltManagerViewExt for Account {
    async fn is_valid_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: &Salt,
    ) -> anyhow::Result<bool> {
        let account = Account::new(defuse_contract_id.clone(), self.network_config().clone());

        account
            .call_view_function_json("is_valid_salt", json!({ "salt": salt }))
            .await
            .map_err(Into::into)
    }

    async fn current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt> {
        let account = Account::new(defuse_contract_id.clone(), self.network_config().clone());

        account
            .call_view_function_json("current_salt", ())
            .await
            .map_err(Into::into)
    }
}
