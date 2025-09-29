use defuse::core::Salt;
use near_sdk::{AccountId, NearToken};
use serde_json::json;

pub trait SaltManagerExt {
    async fn rotate_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<()>;

    async fn invalidate_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: Salt,
    ) -> anyhow::Result<()>;

    async fn is_valid_salt(&self, salt: Salt) -> anyhow::Result<bool>;

    async fn get_current_salt(&self) -> anyhow::Result<Salt>;
}

impl SaltManagerExt for near_workspaces::Account {
    async fn rotate_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "rotate_salt")
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }

    async fn invalidate_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: Salt,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "invalidate_salt")
            .args_json(json!({ "salt": salt }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }

    async fn is_valid_salt(&self, salt: Salt) -> anyhow::Result<bool> {
        self.view(self.id(), "is_valid_salt")
            .args_json(json!({ "salt": salt }))
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn get_current_salt(&self) -> anyhow::Result<Salt> {
        self.view(self.id(), "get_current_salt")
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl SaltManagerExt for near_workspaces::Contract {
    async fn rotate_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<()> {
        self.as_account().rotate_salt(defuse_contract_id).await
    }

    async fn invalidate_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: Salt,
    ) -> anyhow::Result<()> {
        self.as_account()
            .invalidate_salt(defuse_contract_id, salt)
            .await
    }

    async fn is_valid_salt(&self, salt: Salt) -> anyhow::Result<bool> {
        self.as_account().is_valid_salt(salt).await
    }

    async fn get_current_salt(&self) -> anyhow::Result<Salt> {
        self.as_account().get_current_salt().await
    }
}
