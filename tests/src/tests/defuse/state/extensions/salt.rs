use defuse::core::Salt;
use near_sdk::{AccountId, NearToken};
use serde_json::json;

pub trait SaltManagerExt {
    async fn update_current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt>;

    async fn invalidate_salts(
        &self,
        defuse_contract_id: &AccountId,
        salts: &[Salt],
    ) -> anyhow::Result<Salt>;

    async fn is_valid_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: &Salt,
    ) -> anyhow::Result<bool>;

    async fn current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt>;
}

impl SaltManagerExt for near_workspaces::Account {
    async fn update_current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt> {
        self.call(defuse_contract_id, "update_current_salt")
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?
            .json()
            .map_err(Into::into)
    }

    async fn invalidate_salts(
        &self,
        defuse_contract_id: &AccountId,
        salts: &[Salt],
    ) -> anyhow::Result<Salt> {
        self.call(defuse_contract_id, "invalidate_salts")
            .args_json(json!({ "salts": salts }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?
            .json()
            .map_err(Into::into)
    }

    async fn is_valid_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: &Salt,
    ) -> anyhow::Result<bool> {
        self.view(defuse_contract_id, "is_valid_salt")
            .args_json(json!({ "salt": salt }))
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt> {
        self.view(defuse_contract_id, "current_salt")
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl SaltManagerExt for near_workspaces::Contract {
    async fn update_current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt> {
        self.as_account()
            .update_current_salt(defuse_contract_id)
            .await
    }

    async fn invalidate_salts(
        &self,
        defuse_contract_id: &AccountId,
        salts: &[Salt],
    ) -> anyhow::Result<Salt> {
        self.as_account()
            .invalidate_salts(defuse_contract_id, salts)
            .await
    }

    async fn is_valid_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: &Salt,
    ) -> anyhow::Result<bool> {
        self.as_account()
            .is_valid_salt(defuse_contract_id, salt)
            .await
    }

    async fn current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt> {
        self.as_account().current_salt(defuse_contract_id).await
    }
}
