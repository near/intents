use near_contract_standards::storage_management::StorageBalance;
use near_sdk::{AccountId, AccountIdRef, NearToken, serde_json::json};

use crate::{Account, SigningAccount, tx::FnCallBuilder};

#[allow(async_fn_in_trait)]
pub trait StorageManagementExt {
    async fn storage_deposit(
        &self,
        contract_id: &AccountIdRef,
        account_id: Option<&AccountId>,
        deposit: NearToken,
    ) -> anyhow::Result<StorageBalance>;

    async fn storage_withdraw(
        &self,
        contract_id: &AccountIdRef,
        amount: NearToken,
    ) -> anyhow::Result<StorageBalance>;

    async fn storage_unregister(
        &self,
        contract_id: &AccountIdRef,
        force: Option<bool>,
    ) -> anyhow::Result<bool>;
}

#[allow(async_fn_in_trait)]
pub trait StorageViewExt {
    async fn storage_balance_of(
        &self,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<Option<StorageBalance>>;
}

impl StorageManagementExt for SigningAccount {
    async fn storage_deposit(
        &self,
        contract_id: &AccountIdRef,
        account_id: Option<&AccountId>,
        deposit: NearToken,
    ) -> anyhow::Result<StorageBalance> {
        self.tx(contract_id.into())
            .function_call(
                FnCallBuilder::new("storage_deposit")
                    .json_args(&json!({
                        "account_id": account_id.unwrap_or_else(|| self.id())
                    }))
                    .with_deposit(NearToken::from_yoctonear(deposit.as_yoctonear())),
            )
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn storage_withdraw(
        &self,
        contract_id: &AccountIdRef,
        amount: NearToken,
    ) -> anyhow::Result<StorageBalance> {
        self.tx(contract_id.into())
            .function_call(
                FnCallBuilder::new("storage_withdraw")
                    .json_args(&json!({
                        "amount": amount.as_yoctonear()
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn storage_unregister(
        &self,
        contract_id: &AccountIdRef,
        force: Option<bool>,
    ) -> anyhow::Result<bool> {
        self.tx(contract_id.into())
            .function_call(
                FnCallBuilder::new("storage_unregister")
                    .json_args(&json!({
                        "force": force
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl StorageViewExt for Account {
    async fn storage_balance_of(
        &self,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<Option<StorageBalance>> {
        self.call_view_function_json(
            "storage_balance_of",
            json!({
                "account_id": account_id
            }),
        )
        .await
    }
}

impl StorageViewExt for SigningAccount {
    async fn storage_balance_of(
        &self,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<Option<StorageBalance>> {
        self.account().storage_balance_of(account_id).await
    }
}
