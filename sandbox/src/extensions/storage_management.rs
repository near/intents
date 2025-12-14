use near_api::types::json::U128;
use near_contract_standards::storage_management::StorageBalance;
use near_sdk::{AccountId, AccountIdRef, NearToken, serde_json::json};

use crate::{Account, SigningAccount, tx::FnCallBuilder};

pub trait StorageManagementExt {
    async fn storage_deposit(
        &self,
        contract_id: impl Into<AccountId>,
        account_id: impl Into<Option<&AccountIdRef>>,
        deposit: NearToken,
    ) -> anyhow::Result<StorageBalance>;

    async fn storage_withdraw(
        &self,
        contract_id: impl Into<AccountId>,
        amount: NearToken,
    ) -> anyhow::Result<StorageBalance>;

    async fn storage_unregister(
        &self,
        contract_id: impl Into<AccountId>,
        force: impl Into<Option<bool>>,
    ) -> anyhow::Result<bool>;
}
impl StorageManagementExt for SigningAccount {
    async fn storage_deposit(
        &self,
        contract_id: impl Into<AccountId>,
        account_id: impl Into<Option<&AccountIdRef>>,
        deposit: NearToken,
    ) -> anyhow::Result<StorageBalance> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("storage_deposit")
                    .json_args(json!({
                        "account_id": account_id.into(),
                    }))
                    .with_deposit(deposit),
            )
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn storage_withdraw(
        &self,
        contract_id: impl Into<AccountId>,
        amount: NearToken,
    ) -> anyhow::Result<StorageBalance> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("storage_withdraw")
                    .json_args(json!({
                        "amount": U128::from(amount.as_yoctonear())
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn storage_unregister(
        &self,
        contract_id: impl Into<AccountId>,
        force: impl Into<Option<bool>>,
    ) -> anyhow::Result<bool> {
        self.tx(contract_id)
            .function_call(
                FnCallBuilder::new("storage_unregister")
                    .json_args(json!({
                        "force": force.into()
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }
}

pub trait StorageViewExt {
    async fn storage_balance_of(
        &self,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<Option<StorageBalance>>;
}

impl StorageViewExt for Account {
    async fn storage_balance_of(
        &self,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<Option<StorageBalance>> {
        self.call_view_function_json(
            "storage_balance_of",
            json!({
                "account_id": account_id.as_ref()
            }),
        )
        .await
    }
}
