use near_sdk::{AccountIdRef, serde_json::json};

use crate::{
    SigningAccount,
    tx::{FnCallBuilder, TxResult},
};

pub trait AclExt {
    async fn acl_add_super_admin(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> TxResult<()>;

    async fn acl_revoke_super_admin(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> TxResult<()>;

    async fn acl_add_admin(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()>;

    async fn acl_revoke_admin(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()>;

    async fn acl_grant_role(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()>;

    async fn acl_revoke_role(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()>;
}

impl AclExt for SigningAccount {
    async fn acl_add_super_admin(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call(FnCallBuilder::new("acl_add_super_admin").json_args(&json!({
            "account_id": account_id,
            })))
            .await?;

        Ok(())
    }

    async fn acl_revoke_super_admin(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call(
                FnCallBuilder::new("acl_revoke_super_admin").json_args(&json!({
                "account_id": account_id,
                })),
            )
            .await?;

        Ok(())
    }

    async fn acl_add_admin(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call(FnCallBuilder::new("acl_add_admin").json_args(&json!({
                "role": role.into(),
                "account_id": account_id,                })))
            .await?;

        Ok(())
    }

    async fn acl_revoke_admin(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call(FnCallBuilder::new("acl_revoke_admin").json_args(&json!({
                "role": role.into(),
                "account_id": account_id,
            })))
            .await?;

        Ok(())
    }

    async fn acl_grant_role(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call(FnCallBuilder::new("acl_grant_role").json_args(&json!({
                "role": role.into(),
                "account_id": account_id,
            })))
            .await?;

        Ok(())
    }

    async fn acl_revoke_role(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call(FnCallBuilder::new("acl_revoke_role").json_args(&json!({
                "role": role.into(),
                "account_id": account_id,
            })))
            .await?;

        Ok(())
    }
}
