use crate::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use near_api::types::transaction::result::ExecutionSuccess;
use near_sdk::{AccountId, AccountIdRef, NearToken, serde_json::json};

pub trait ForceAccountManagerExt {
    async fn force_lock_account(
        &self,
        contract_id: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<(ExecutionSuccess, bool)>;

    async fn force_unlock_account(
        &self,
        contract_id: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<(ExecutionSuccess, bool)>;

    async fn force_disable_auth_by_predecessor_ids(
        &self,
        contract_id: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn force_enable_auth_by_predecessor_ids(
        &self,
        contract_id: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> anyhow::Result<ExecutionSuccess>;
}

pub trait ForceAccountViewExt {
    async fn is_account_locked(&self, account_id: impl AsRef<AccountIdRef>)
    -> anyhow::Result<bool>;
}

impl ForceAccountManagerExt for SigningAccount {
    async fn force_lock_account(
        &self,
        contract_id: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<(ExecutionSuccess, bool)> {
        let res = self
            .tx(contract_id)
            .function_call(
                FnCallBuilder::new("force_lock_account")
                    .json_args(json!({
                        "account_id": account_id.as_ref(),
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        let locked = res.json()?;

        Ok((res, locked))
    }

    async fn force_unlock_account(
        &self,
        contract_id: impl Into<AccountId>,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<(ExecutionSuccess, bool)> {
        let res = self
            .tx(contract_id)
            .function_call(
                FnCallBuilder::new("force_unlock_account")
                    .json_args(json!({
                        "account_id": account_id.as_ref(),
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        let unlocked = res.json()?;

        Ok((res, unlocked))
    }

    async fn force_disable_auth_by_predecessor_ids(
        &self,
        contract_id: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("force_disable_auth_by_predecessor_ids")
                    .json_args(json!({
                        "account_ids": account_ids.into_iter().collect::<Vec<_>>(),
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
    }

    async fn force_enable_auth_by_predecessor_ids(
        &self,
        contract_id: impl Into<AccountId>,
        account_ids: impl IntoIterator<Item = AccountId>,
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("force_enable_auth_by_predecessor_ids")
                    .json_args(json!({
                        "account_ids": account_ids.into_iter().collect::<Vec<_>>(),
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
    }
}

impl ForceAccountViewExt for Account {
    async fn is_account_locked(
        &self,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<bool> {
        self.call_view_function_json(
            "is_account_locked",
            json!({
                "account_id": account_id.as_ref(),
            }),
        )
        .await
    }
}
