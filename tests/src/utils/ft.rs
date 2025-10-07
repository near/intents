use near_contract_standards::storage_management::StorageBalance;
use near_sdk::json_types::U128;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, AccountId, Contract};
use serde_json::json;

use super::account::AccountExt;
use super::storage_management::StorageManagementExt;
use super::test_log::TestLog;

pub const FT_STORAGE_DEPOSIT: NearToken = NearToken::from_yoctonear(2_350_000_000_000_000_000_000);
const TOTAL_SUPPLY: u128 = 1_000_000_000;

const FUNGIBLE_TOKEN_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/contracts/fungible-token.wasm"
));

pub trait FtExt: StorageManagementExt {
    async fn deploy_vanilla_ft_token(&self, token_name: &str) -> anyhow::Result<Contract>;

    async fn ft_token_balance_of(
        &self,
        token_id: &AccountId,
        account_id: &AccountId,
    ) -> anyhow::Result<u128>;

    async fn ft_balance_of(&self, account_id: &AccountId) -> anyhow::Result<u128>;

    async fn ft_transfer(
        &self,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
    ) -> anyhow::Result<()>;

    async fn ft_transfer_call(
        &self,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
        msg: &str,
    ) -> anyhow::Result<u128>;

    async fn ft_storage_deposit(
        &self,
        token_id: &AccountId,
        account_id: Option<&AccountId>,
    ) -> anyhow::Result<StorageBalance> {
        self.storage_deposit(token_id, account_id, FT_STORAGE_DEPOSIT)
            .await
    }
    async fn ft_storage_deposit_many(
        &self,
        token_id: &AccountId,
        accounts: &[&AccountId],
    ) -> anyhow::Result<()>;
}

impl FtExt for Account {
    async fn deploy_vanilla_ft_token(&self, token_name: &str) -> anyhow::Result<Contract> {
        let contract = self
            .deploy_contract(token_name, FUNGIBLE_TOKEN_WASM)
            .await?;
        contract
            .call("new")
            .args_json(json!({
                "owner_id": self.id(),
                "total_supply": TOTAL_SUPPLY.to_string(),
                "metadata": {
                    "spec": "ft-1.0.0",
                    "name": format!("Token {}", token_name),
                    "symbol": "TKN",
                    "decimals": 18
                }
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(contract)
    }

    async fn ft_token_balance_of(
        &self,
        token_id: &AccountId,
        account_id: &AccountId,
    ) -> anyhow::Result<u128> {
        self.view(token_id, "ft_balance_of")
            .args_json(json!({
                "account_id": account_id,
            }))
            .await?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }

    async fn ft_balance_of(&self, account_id: &AccountId) -> anyhow::Result<u128> {
        self.ft_token_balance_of(self.id(), account_id).await
    }

    async fn ft_transfer(
        &self,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.call(token_id, "ft_transfer")
            .args_json(json!({
                "receiver_id": receiver_id,
                "amount": U128(amount),
                "memo": memo,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn ft_transfer_call(
        &self,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
        msg: &str,
    ) -> anyhow::Result<u128> {
        self.call(token_id, "ft_transfer_call")
            .args_json(json!({
                "receiver_id": receiver_id,
                "amount": U128(amount),
                "memo": memo,
                "msg": msg,
            }))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?
            .into_result()
            .inspect(|outcome| {
                println!(
                    "ft_transfer_call Logs: {:?}",
                    TestLog::from(outcome.clone())
                );
            })?
            .json::<U128>()
            .map(|v| v.0)
            .map_err(Into::into)
    }

    async fn ft_storage_deposit_many(
        &self,
        token_id: &AccountId,
        accounts: &[&AccountId],
    ) -> anyhow::Result<()> {
        for account in accounts {
            self.ft_storage_deposit(token_id, Some(account)).await?;
        }
        Ok(())
    }
}

impl FtExt for Contract {
    async fn deploy_vanilla_ft_token(&self, token_name: &str) -> anyhow::Result<Self> {
        self.as_account().deploy_vanilla_ft_token(token_name).await
    }

    async fn ft_token_balance_of(
        &self,
        token_id: &AccountId,
        account_id: &AccountId,
    ) -> anyhow::Result<u128> {
        self.as_account()
            .ft_token_balance_of(token_id, account_id)
            .await
    }

    async fn ft_balance_of(&self, account_id: &AccountId) -> anyhow::Result<u128> {
        self.as_account().ft_balance_of(account_id).await
    }

    async fn ft_transfer(
        &self,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        self.as_account()
            .ft_transfer(token_id, receiver_id, amount, memo)
            .await
    }

    async fn ft_transfer_call(
        &self,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
        msg: &str,
    ) -> anyhow::Result<u128> {
        self.as_account()
            .ft_transfer_call(token_id, receiver_id, amount, memo, msg)
            .await
    }

    async fn ft_storage_deposit_many(
        &self,
        token_id: &AccountId,
        accounts: &[&AccountId],
    ) -> anyhow::Result<()> {
        for account in accounts {
            self.ft_storage_deposit(token_id, Some(account)).await?;
        }
        Ok(())
    }
}
