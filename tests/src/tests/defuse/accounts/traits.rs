use near_sdk::{AccountId, AccountIdRef, NearToken};
use serde_json::json;

pub trait AccountForceLockerExt {
    async fn is_account_locked(
        &self,
        contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool>;

    async fn force_lock_account(
        &self,
        contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool>;

    async fn force_unlock_account(
        &self,
        contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool>;
}

impl AccountForceLockerExt for near_workspaces::Account {
    async fn is_account_locked(
        &self,
        contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.view(contract_id, "is_account_locked")
            .args_json(json!({
                "account_id": account_id,
            }))
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn force_lock_account(
        &self,
        contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.call(contract_id, "force_lock_account")
            .args_json(json!({
                "account_id": account_id,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?
            .json()
            .map_err(Into::into)
    }

    async fn force_unlock_account(
        &self,
        contract_id: &AccountId,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.call(contract_id, "force_unlock_account")
            .args_json(json!({
                "account_id": account_id,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?
            .json()
            .map_err(Into::into)
    }
}
