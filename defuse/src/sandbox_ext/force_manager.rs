use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountId, AccountIdRef, NearToken, serde_json::json};

#[allow(async_fn_in_trait)]
pub trait ForceAccountManagerExt {
    async fn force_lock_account(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool>;

    async fn force_unlock_account(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool>;

    async fn force_disable_auth_by_predecessor_ids(
        &self,
        contract_id: &AccountIdRef,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> anyhow::Result<()>;

    async fn force_enable_auth_by_predecessor_ids(
        &self,
        contract_id: &AccountIdRef,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> anyhow::Result<()>;
}

#[allow(async_fn_in_trait)]
pub trait ForceAccountViewExt {
    async fn is_account_locked(&self, account_id: &AccountIdRef) -> anyhow::Result<bool>;
}

impl ForceAccountManagerExt for SigningAccount {
    async fn force_lock_account(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.tx(contract_id.into())
            .function_call(
                FnCallBuilder::new("force_lock_account")
                    .json_args(json!({
                        "account_id": account_id,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn force_unlock_account(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<bool> {
        self.tx(contract_id.into())
            .function_call(
                FnCallBuilder::new("force_unlock_account")
                    .json_args(json!({
                        "account_id": account_id,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn force_disable_auth_by_predecessor_ids(
        &self,
        contract_id: &AccountIdRef,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> anyhow::Result<()> {
        self.tx(contract_id.into())
            .function_call(
                FnCallBuilder::new("force_disable_auth_by_predecessor_ids")
                    .json_args(json!({
                        "account_ids": account_ids.into_iter().collect::<Vec<_>>(),
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(())
    }

    async fn force_enable_auth_by_predecessor_ids(
        &self,
        contract_id: &AccountIdRef,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> anyhow::Result<()> {
        self.tx(contract_id.into())
            .function_call(
                FnCallBuilder::new("force_enable_auth_by_predecessor_ids")
                    .json_args(json!({
                        "account_ids": account_ids.into_iter().collect::<Vec<_>>(),
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(())
    }
}

impl ForceAccountViewExt for Account {
    async fn is_account_locked(&self, account_id: &AccountIdRef) -> anyhow::Result<bool> {
        self.call_view_function_json(
            "is_account_locked",
            json!({
                "account_id": account_id,
            }),
        )
        .await
    }
}
