use defuse::core::Salt;
use near_sdk::{AccountId, NearToken};
use serde_json::json;

pub trait SaltManagerExt {
    async fn rotate_salt(
        &self,
        defuse_contract_id: &AccountId,
        invalidate_current: bool,
    ) -> anyhow::Result<Salt>;

    async fn invalidate_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: &Salt,
    ) -> anyhow::Result<Salt>;

    async fn is_valid_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: &Salt,
    ) -> anyhow::Result<bool>;

    async fn get_current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt>;
}

impl SaltManagerExt for near_workspaces::Account {
    async fn rotate_salt(
        &self,
        defuse_contract_id: &AccountId,
        invalidate_current: bool,
    ) -> anyhow::Result<Salt> {
        self.call(defuse_contract_id, "rotate_salt")
            .args_json(json!({ "invalidate_current": invalidate_current }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?
            .json()
            .map_err(Into::into)
    }

    async fn invalidate_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: &Salt,
    ) -> anyhow::Result<Salt> {
        self.call(defuse_contract_id, "invalidate_salt")
            .args_json(json!({ "salt": salt }))
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

    async fn get_current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt> {
        self.view(defuse_contract_id, "get_current_salt")
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl SaltManagerExt for near_workspaces::Contract {
    async fn rotate_salt(
        &self,
        defuse_contract_id: &AccountId,
        invalidate_current: bool,
    ) -> anyhow::Result<Salt> {
        self.as_account()
            .rotate_salt(defuse_contract_id, invalidate_current)
            .await
    }

    async fn invalidate_salt(
        &self,
        defuse_contract_id: &AccountId,
        salt: &Salt,
    ) -> anyhow::Result<Salt> {
        self.as_account()
            .invalidate_salt(defuse_contract_id, salt)
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

    async fn get_current_salt(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Salt> {
        self.as_account().get_current_salt(defuse_contract_id).await
    }
}
