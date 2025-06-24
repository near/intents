mod auth_by_predecessor_id;
mod locked;
mod traits;

use defuse::core::{Nonce, crypto::PublicKey};
use defuse_serde_utils::base64::AsBase64;
use near_sdk::{AccountId, AccountIdRef, Gas, NearToken};
use serde_json::json;

pub trait AccountManagerExt {
    async fn add_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()>;

    async fn remove_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()>;

    async fn defuse_has_public_key(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool>;

    async fn has_public_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool>;

    async fn is_nonce_used(&self, account_id: &AccountId, nonce: &Nonce) -> anyhow::Result<bool>;

    async fn is_auth_by_predecessor_id_enabled(
        &self,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool>;

    async fn disable_auth_by_predecessor_id(
        &self,
        defuse_contract_id: &AccountId,
    ) -> anyhow::Result<()>;
}

impl AccountManagerExt for near_workspaces::Account {
    async fn add_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        // TODO: check bool output
        self.call(defuse_contract_id, "add_public_key")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "public_key": public_key,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn remove_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "remove_public_key")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "public_key": public_key,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn defuse_has_public_key(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.view(defuse_contract_id, "has_public_key")
            .args_json(json!({
                "account_id": account_id,
                "public_key": public_key,
            }))
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn has_public_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.defuse_has_public_key(self.id(), account_id, public_key)
            .await
    }

    async fn is_nonce_used(&self, account_id: &AccountId, nonce: &Nonce) -> anyhow::Result<bool> {
        self.view(self.id(), "is_nonce_used")
            .args_json(json!({
                "account_id": account_id,
                "nonce": AsBase64(nonce),
            }))
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn is_auth_by_predecessor_id_enabled(
        &self,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.view(self.id(), "is_auth_by_predecessor_id_enabled")
            .args_json(json!({
                "account_id": account_id,
            }))
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn disable_auth_by_predecessor_id(
        &self,
        defuse_contract_id: &AccountId,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "disable_auth_by_predecessor_id")
            .deposit(NearToken::from_yoctonear(1))
            .gas(Gas::from_tgas(10))
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }
}

impl AccountManagerExt for near_workspaces::Contract {
    async fn add_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.as_account()
            .add_public_key(defuse_contract_id, public_key)
            .await
    }

    async fn remove_public_key(
        &self,
        defuse_contract_id: &AccountId,
        public_key: PublicKey,
    ) -> anyhow::Result<()> {
        self.as_account()
            .remove_public_key(defuse_contract_id, public_key)
            .await
    }

    async fn defuse_has_public_key(
        &self,
        defuse_contract_id: &AccountId,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.as_account()
            .defuse_has_public_key(defuse_contract_id, account_id, public_key)
            .await
    }

    async fn has_public_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> anyhow::Result<bool> {
        self.as_account()
            .has_public_key(account_id, public_key)
            .await
    }

    async fn is_nonce_used(&self, account_id: &AccountId, nonce: &Nonce) -> anyhow::Result<bool> {
        self.as_account().is_nonce_used(account_id, nonce).await
    }

    async fn is_auth_by_predecessor_id_enabled(
        &self,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.as_account()
            .is_auth_by_predecessor_id_enabled(account_id)
            .await
    }

    async fn disable_auth_by_predecessor_id(
        &self,
        defuse_contract_id: &AccountId,
    ) -> anyhow::Result<()> {
        self.as_account()
            .disable_auth_by_predecessor_id(defuse_contract_id)
            .await
    }
}
