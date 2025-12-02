use near_contract_standards::storage_management::StorageBalance;
use near_sdk::{AccountId, AccountIdRef, Gas, NearToken, serde_json::json};

use crate::{Account, SigningAccount, TxResult};

pub trait StorageManagementExt {
    async fn storage_deposit(
        &self,
        contract_id: &AccountIdRef,
        account_id: Option<&AccountId>,
        deposit: NearToken,
    ) -> TxResult<StorageBalance>;

    async fn storage_withdraw(
        &self,
        contract_id: &AccountIdRef,
        amount: NearToken,
    ) -> TxResult<StorageBalance>;

    async fn storage_unregister(
        &self,
        contract_id: &AccountIdRef,
        force: Option<bool>,
    ) -> TxResult<bool>;
}

pub trait StorageViewExt {
    async fn storage_balance_of(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<Option<StorageBalance>>;
}

impl StorageManagementExt for SigningAccount {
    async fn storage_deposit(
        &self,
        contract_id: &AccountIdRef,
        account_id: Option<&AccountId>,
        deposit: NearToken,
    ) -> TxResult<StorageBalance> {
        self.tx(contract_id.into())
            .function_call_json(
                "storage_deposit",
                json!({
                    "account_id": account_id.unwrap_or_else(|| self.id())
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(deposit.as_yoctonear()),
            )
            .await
    }

    async fn storage_withdraw(
        &self,
        contract_id: &AccountIdRef,
        amount: NearToken,
    ) -> TxResult<StorageBalance> {
        self.tx(contract_id.into())
            .function_call_json(
                "storage_withdraw",
                json!({
                    "amount": amount.as_yoctonear()
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(1),
            )
            .await
    }

    async fn storage_unregister(
        &self,
        contract_id: &AccountIdRef,
        force: Option<bool>,
    ) -> TxResult<bool> {
        self.tx(contract_id.into())
            .function_call_json(
                "storage_unregister",
                json!({
                    "force": force
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(1),
            )
            .await
    }
}

impl StorageViewExt for Account {
    async fn storage_balance_of(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<Option<StorageBalance>> {
        let account = Account::new(contract_id.into(), self.network_config().clone());

        account
            .call_view_function_json(
                "storage_balance_of",
                json!({
                    "account_id": account_id
                }),
            )
            .await
    }
}

// TODO: make all rest
impl StorageViewExt for SigningAccount {
    async fn storage_balance_of(
        &self,
        contract_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<Option<StorageBalance>> {
        self.account()
            .storage_balance_of(contract_id, account_id)
            .await
    }
}
