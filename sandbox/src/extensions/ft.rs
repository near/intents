use near_contract_standards::storage_management::StorageBalance;
use near_sdk::{AccountId, AccountIdRef, Gas, NearToken, json_types::U128, serde_json::json};

use crate::{
    Account, SigningAccount,
    extensions::storage_management::StorageManagementExt,
    tx::{FnCallBuilder, TxResult},
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
            .function_call(
                FnCallBuilder::new("ft_transfer")
                    .json_args(&json!({
                        "receiver_id": receiver_id,
                        "amount": U128(amount),
                        "memo": memo,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(())
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
            .function_call(
                FnCallBuilder::new("ft_transfer_call")
                    .json_args(&json!({
                        "receiver_id": receiver_id,
                        "amount": U128(amount),
                        "memo": memo,
                        "msg": msg,
                    }))
                    .with_gas(Gas::from_tgas(300))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
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
