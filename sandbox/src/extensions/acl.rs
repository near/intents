use near_sdk::{AccountIdRef, Gas, NearToken, serde_json::json};

use crate::{SigningAccount, TxResult};

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
            .function_call_json(
                "acl_add_super_admin",
                json!({
                "account_id": account_id,
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(0),
            )
            .await
    }

    async fn acl_revoke_super_admin(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call_json(
                "acl_revoke_super_admin",
                json!({
                "account_id": account_id,
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(0),
            )
            .await
    }

    async fn acl_add_admin(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call_json(
                "acl_add_admin",
                json!({
                "role": role.into(),
                "account_id": account_id,                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(0),
            )
            .await
    }

    async fn acl_revoke_admin(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call_json(
                "acl_revoke_admin",
                json!({
                "role": role.into(),
                "account_id": account_id,                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(0),
            )
            .await
    }

    async fn acl_grant_role(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call_json(
                "acl_grant_role",
                json!({
                "role": role.into(),
                "account_id": account_id,                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(0),
            )
            .await
    }

    async fn acl_revoke_role(
        &self,
        contract_id: &AccountIdRef,
        role: impl Into<String>,
        account_id: &AccountIdRef,
    ) -> TxResult<()> {
        self.tx(contract_id.into())
            .function_call_json(
                "acl_revoke_role",
                json!({
                "role": role.into(),
                "account_id": account_id,                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(0),
            )
            .await
    }
}
