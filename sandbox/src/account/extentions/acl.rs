use near_sdk::AccountId;
use near_workspaces::Account;
use serde_json::json;

pub trait AclExt {
    async fn acl_add_super_admin(
        &self,
        contract_id: &AccountId,
        account_id: &AccountId,
    ) -> anyhow::Result<()>;
    async fn acl_revoke_super_admin(
        &self,
        contract_id: &AccountId,
        account_id: &AccountId,
    ) -> anyhow::Result<()>;

    async fn acl_add_admin(
        &self,
        contract_id: &AccountId,
        role: impl Into<String>,
        account_id: &AccountId,
    ) -> anyhow::Result<()>;
    async fn acl_revoke_admin(
        &self,
        contract_id: &AccountId,
        role: impl Into<String>,
        account_id: &AccountId,
    ) -> anyhow::Result<()>;

    async fn acl_grant_role(
        &self,
        contract_id: &AccountId,
        role: impl Into<String>,
        account_id: &AccountId,
    ) -> anyhow::Result<()>;
    async fn acl_revoke_role(
        &self,
        contract_id: &AccountId,
        role: impl Into<String>,
        account_id: &AccountId,
    ) -> anyhow::Result<()>;
}

impl AclExt for Account {
    async fn acl_add_super_admin(
        &self,
        contract_id: &AccountId,
        account_id: &AccountId,
    ) -> anyhow::Result<()> {
        self.call(contract_id, "acl_add_super_admin")
            .args_json(json!({
                "account_id": account_id,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn acl_revoke_super_admin(
        &self,
        contract_id: &AccountId,
        account_id: &AccountId,
    ) -> anyhow::Result<()> {
        self.call(contract_id, "acl_revoke_super_admin")
            .args_json(json!({
                "account_id": account_id,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn acl_add_admin(
        &self,
        contract_id: &AccountId,
        role: impl Into<String>,
        account_id: &AccountId,
    ) -> anyhow::Result<()> {
        self.call(contract_id, "acl_add_admin")
            .args_json(json!({
                "role": role.into(),
                "account_id": account_id,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn acl_revoke_admin(
        &self,
        contract_id: &AccountId,
        role: impl Into<String>,
        account_id: &AccountId,
    ) -> anyhow::Result<()> {
        self.call(contract_id, "acl_revoke_admin")
            .args_json(json!({
                "role": role.into(),
                "account_id": account_id,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn acl_grant_role(
        &self,
        contract_id: &AccountId,
        role: impl Into<String>,
        account_id: &AccountId,
    ) -> anyhow::Result<()> {
        self.call(contract_id, "acl_grant_role")
            .args_json(json!({
                "role": role.into(),
                "account_id": account_id,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn acl_revoke_role(
        &self,
        contract_id: &AccountId,
        role: impl Into<String>,
        account_id: &AccountId,
    ) -> anyhow::Result<()> {
        self.call(contract_id, "acl_revoke_role")
            .args_json(json!({
                "role": role.into(),
                "account_id": account_id,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }
}
