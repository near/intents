use near_api::{Account as NearApiAccount, PublicKey, types::AccessKey};
use near_sdk::{AccountIdRef, serde_json::json};

use crate::{Account, SigningAccount, tx::FnCallBuilder};

#[allow(async_fn_in_trait)]
pub trait AclViewExt {
    async fn view_access_keys(&self) -> anyhow::Result<Vec<(PublicKey, AccessKey)>>;
}

#[allow(async_fn_in_trait)]
pub trait AclExt {
    async fn acl_add_super_admin(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()>;

    async fn acl_revoke_super_admin(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()>;

    async fn acl_add_admin(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()>;

    async fn acl_revoke_admin(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()>;

    async fn acl_grant_role(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()>;

    async fn acl_revoke_role(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()>;
}

impl AclViewExt for Account {
    async fn view_access_keys(&self) -> anyhow::Result<Vec<(PublicKey, AccessKey)>> {
        NearApiAccount(self.id().clone())
            .list_keys()
            .fetch_from(self.network_config())
            .await
            .map(|d| d.data)
            .map_err(Into::into)
    }
}

impl AclExt for SigningAccount {
    async fn acl_add_super_admin(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()> {
        self.tx(contract_id.as_ref().into())
            .function_call(FnCallBuilder::new("acl_add_super_admin").json_args(json!({
            "account_id": account_id.as_ref(),
            })))
            .await?;

        Ok(())
    }

    async fn acl_revoke_super_admin(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()> {
        self.tx(contract_id.as_ref().into())
            .function_call(
                FnCallBuilder::new("acl_revoke_super_admin").json_args(json!({
                "account_id": account_id.as_ref(),
                })),
            )
            .await?;

        Ok(())
    }

    async fn acl_add_admin(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()> {
        self.tx(contract_id.as_ref().into())
            .function_call(FnCallBuilder::new("acl_add_admin").json_args(json!({
                "role": role.into(),
                "account_id": account_id.as_ref(),                })))
            .await?;

        Ok(())
    }

    async fn acl_revoke_admin(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()> {
        self.tx(contract_id.as_ref().into())
            .function_call(FnCallBuilder::new("acl_revoke_admin").json_args(json!({
                "role": role.into(),
                "account_id": account_id.as_ref(),
            })))
            .await?;

        Ok(())
    }

    async fn acl_grant_role(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()> {
        self.tx(contract_id.as_ref().into())
            .function_call(FnCallBuilder::new("acl_grant_role").json_args(json!({
                "role": role.into(),
                "account_id": account_id.as_ref(),
            })))
            .await?;

        Ok(())
    }

    async fn acl_revoke_role(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        role: impl Into<String>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()> {
        self.tx(contract_id.as_ref().into())
            .function_call(FnCallBuilder::new("acl_revoke_role").json_args(json!({
                "role": role.into(),
                "account_id": account_id.as_ref(),
            })))
            .await?;

        Ok(())
    }
}
