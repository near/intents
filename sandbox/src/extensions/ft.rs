use near_api::types::errors::{DataConversionError, ExecutionError};
use near_contract_standards::storage_management::StorageBalance;
use near_sdk::{AccountId, AccountIdRef, Gas, NearToken, json_types::U128, serde_json::json};

use crate::{
    Account, SigningAccount, TxResult, extensions::storage_management::StorageManagementExt,
};

pub const FT_STORAGE_DEPOSIT: NearToken = NearToken::from_yoctonear(2_350_000_000_000_000_000_000);

pub trait FtExt: StorageManagementExt {
    async fn ft_transfer(
        &self,
        token_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
    ) -> TxResult<()>;

    async fn ft_transfer_call(
        &self,
        token_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
        msg: &str,
    ) -> TxResult<u128>;

    async fn ft_storage_deposit(
        &self,
        token_id: &AccountIdRef,
        account_id: Option<&AccountId>,
    ) -> TxResult<StorageBalance> {
        self.storage_deposit(token_id, account_id, FT_STORAGE_DEPOSIT)
            .await
    }
}

pub trait FtViewExt {
    async fn ft_balance_of(
        &self,
        token_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<u128>;
}

impl FtExt for SigningAccount {
    async fn ft_transfer(
        &self,
        token_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
    ) -> TxResult<()> {
        self.tx(token_id.into())
            .function_call_json(
                "ft_transfer",
                json!({
                    "receiver_id": receiver_id,
                    "amount": U128(amount),
                    "memo": memo,
                }),
                Gas::from_tgas(15),
                NearToken::from_yoctonear(1),
            )
            .await
    }

    async fn ft_transfer_call(
        &self,
        token_id: &AccountIdRef,
        receiver_id: &AccountIdRef,
        amount: u128,
        memo: Option<String>,
        msg: &str,
    ) -> TxResult<u128> {
        self.tx(token_id.into())
            .function_call_json::<Vec<U128>>(
                "ft_transfer_call",
                json!({
                            "receiver_id": receiver_id,
                "amount": U128(amount),
                "memo": memo,
                "msg": msg,
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(1),
            )
            .await
            .and_then(|amounts| {
                let [amount] = amounts
                    .try_into()
                    .map_err(|amounts: Vec<_>| DataConversionError::IncorrectLength(amounts.len()))
                    .map_err(Into::<ExecutionError>::into)?;
                Ok(amount.0)
            })
    }
}

impl FtViewExt for Account {
    async fn ft_balance_of(
        &self,
        token_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<u128> {
        let account = Account::new(token_id.into(), self.network_config().clone());

        account
            .call_view_function_json::<U128>(
                "ft_balance_of",
                json!({
                    "account_id": account_id,
                }),
            )
            .await
            .map(|v| v.0)
    }
}

impl FtViewExt for SigningAccount {
    async fn ft_balance_of(
        &self,
        token_id: &AccountIdRef,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<u128> {
        self.account().ft_balance_of(token_id, account_id).await
    }
}
