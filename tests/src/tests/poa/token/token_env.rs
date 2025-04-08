use crate::utils::account::AccountExt;
use near_contract_standards::{
    fungible_token::metadata::FungibleTokenMetadata, storage_management::StorageBalance,
};
use near_sdk::{AccountId, AccountIdRef, NearToken, json_types::U128};
use near_workspaces::{Contract, result::ExecutionResult};
use serde_json::json;

// FIXME: reuse the traits of FT from other tests
// FIXME: pass contract id instead of contract

pub const MIN_FT_STORAGE_DEPOSIT_VALUE: NearToken =
    NearToken::from_yoctonear(1_250_000_000_000_000_000_000);

#[cfg(not(clippy))]
pub const POA_TOKEN_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../res/poa-token-with-deposit/defuse_poa_token.wasm"
));
#[cfg(clippy)]
pub const POA_TOKEN_WASM: &[u8] = b"";

pub trait PoATokenExt {
    async fn deploy_poa_token(
        &self,
        id: &str,
        owner_id: Option<&AccountIdRef>,
        metadata: Option<FungibleTokenMetadata>,
    ) -> anyhow::Result<PoATokenContract>;
}

impl PoATokenExt for near_workspaces::Account {
    async fn deploy_poa_token(
        &self,
        id: &str,
        owner_id: Option<&AccountIdRef>,
        metadata: Option<FungibleTokenMetadata>,
    ) -> anyhow::Result<PoATokenContract> {
        let contract = self.deploy_contract(id, POA_TOKEN_WASM).await?;
        let mut json_args = serde_json::Map::new();
        if let Some(oid) = owner_id {
            json_args.insert("owner_id".to_string(), serde_json::to_value(oid).unwrap());
        }
        if let Some(md) = metadata {
            json_args.insert("metadata".to_string(), serde_json::to_value(md).unwrap());
        }

        contract
            .call("new")
            .args_json(json_args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(PoATokenContract::new(contract))
    }
}

pub struct PoATokenContract {
    contract: Contract,
}

impl PoATokenContract {
    fn new(contract: Contract) -> Self {
        Self { contract }
    }

    pub fn id(&self) -> &AccountId {
        self.contract.id()
    }

    pub async fn poa_ft_balance_of(&self, account_id: &AccountIdRef) -> anyhow::Result<U128> {
        self.contract
            .call("ft_balance_of")
            .args_json(json!(
                {
                    "account_id": account_id,
                }
            ))
            .view()
            .await?
            .json()
            .map_err(Into::into)
    }

    pub async fn poa_ft_total_supply(&self) -> anyhow::Result<U128> {
        self.contract
            .call("ft_total_supply")
            .view()
            .await?
            .json()
            .map_err(Into::into)
    }

    pub async fn poa_wrapped_token(&self) -> anyhow::Result<Option<AccountId>> {
        self.contract
            .call("wrapped_token")
            .view()
            .await?
            .json()
            .map_err(Into::into)
    }

    pub async fn poa_ft_metadata(&self) -> anyhow::Result<FungibleTokenMetadata> {
        self.contract
            .call("ft_metadata")
            .view()
            .await?
            .json()
            .map_err(Into::into)
    }
}

pub trait PoATokenContractCaller {
    async fn poa_ft_deposit(
        &self,
        contract: &PoATokenContract,
        owner_id: &AccountIdRef,
        amount: U128,
        memo: Option<String>,
    ) -> anyhow::Result<()>;

    async fn poa_storage_deposit(
        &self,
        contract: &PoATokenContract,
        attached_deposit: NearToken,
        account_id: Option<&AccountIdRef>,
        registration_only: Option<bool>,
    ) -> anyhow::Result<StorageBalance>;

    async fn poa_storage_deposit_simple(
        &self,
        contract: &PoATokenContract,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<StorageBalance>;

    async fn poa_ft_transfer(
        &self,
        contract: &PoATokenContract,
        receiver_id: &AccountIdRef,
        amount: U128,
        memo: Option<String>,
    ) -> anyhow::Result<TestLog>;

    async fn poa_set_wrapped_token_account_id(
        &self,
        contract: &PoATokenContract,
        token_account_id: &AccountIdRef,
        attached_deposit: NearToken,
    ) -> anyhow::Result<TestLog>;

    async fn poa_ft_transfer_call(
        &self,
        contract: &PoATokenContract,
        receiver_id: &AccountIdRef,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<TestLog>;

    async fn poa_force_sync_wrapped_token_metadata(
        &self,
        contract: &PoATokenContract,
        attached_deposit: NearToken,
    ) -> anyhow::Result<TestLog>;

    async fn poa_lock_contract_for_wrapping(&self, contract: &AccountId)
    -> anyhow::Result<TestLog>;

    async fn poa_unlock_contract_for_wrapping(
        &self,
        contract: &AccountId,
    ) -> anyhow::Result<TestLog>;
}

impl PoATokenContractCaller for near_workspaces::Account {
    async fn poa_ft_deposit(
        &self,
        contract: &PoATokenContract,
        owner_id: &AccountIdRef,
        amount: U128,
        memo: Option<String>,
    ) -> anyhow::Result<()> {
        let mut json_args = json!(
            {
                "owner_id": owner_id,
                "amount": amount,
            }
        );

        if let Some(m) = memo {
            json_args
                .as_object_mut()
                .unwrap()
                .insert("memo".to_string(), m.into());
        }

        self.call(contract.contract.id(), "ft_deposit")
            .args_json(json_args)
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }

    async fn poa_storage_deposit_simple(
        &self,
        contract: &PoATokenContract,
        account_id: &AccountIdRef,
    ) -> anyhow::Result<StorageBalance> {
        self.poa_storage_deposit(
            contract,
            MIN_FT_STORAGE_DEPOSIT_VALUE,
            Some(account_id),
            None,
        )
        .await
    }

    async fn poa_storage_deposit(
        &self,
        contract: &PoATokenContract,
        attached_deposit: NearToken,
        account_id: Option<&AccountIdRef>,
        registration_only: Option<bool>,
    ) -> anyhow::Result<StorageBalance> {
        let mut json_args = json!({});

        if let Some(aid) = account_id {
            json_args
                .as_object_mut()
                .unwrap()
                .insert("account_id".to_string(), serde_json::to_value(aid).unwrap());
        }

        if let Some(ro) = registration_only {
            json_args.as_object_mut().unwrap().insert(
                "registration_only".to_string(),
                serde_json::to_value(ro).unwrap(),
            );
        }

        self.call(contract.contract.id(), "storage_deposit")
            .deposit(attached_deposit)
            .args_json(json_args)
            .max_gas()
            .transact()
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn poa_ft_transfer(
        &self,
        contract: &PoATokenContract,
        receiver_id: &AccountIdRef,
        amount: U128,
        memo: Option<String>,
    ) -> anyhow::Result<TestLog> {
        let mut json_args = json!(
            {
                "receiver_id": receiver_id,
                "amount": amount,
            }
        );

        if let Some(m) = memo {
            json_args
                .as_object_mut()
                .unwrap()
                .insert("memo".to_string(), m.into());
        }

        let logs = self
            .call(contract.id(), "ft_transfer")
            .args_json(json_args)
            .max_gas()
            .deposit(NearToken::from_yoctonear(1))
            .transact()
            .await?
            .into_result()
            .map(Into::into)?;

        Ok(logs)
    }

    async fn poa_set_wrapped_token_account_id(
        &self,
        contract: &PoATokenContract,
        token_account_id: &AccountIdRef,
        attached_deposit: NearToken,
    ) -> anyhow::Result<TestLog> {
        let logs = self
            .call(contract.id(), "set_wrapped_token_account_id")
            .args_json(json!(
                {
                    "token_account_id": token_account_id,
                }
            ))
            .max_gas()
            .deposit(attached_deposit)
            .transact()
            .await?
            .into_result()
            .map(Into::into)?;

        Ok(logs)
    }

    async fn poa_ft_transfer_call(
        &self,
        contract: &PoATokenContract,
        receiver_id: &AccountIdRef,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> anyhow::Result<TestLog> {
        let mut json_args = json!(
            {
                "receiver_id": receiver_id,
                "amount": amount,
                "msg": msg,
            }
        );

        if let Some(m) = memo {
            json_args
                .as_object_mut()
                .unwrap()
                .insert("memo".to_string(), m.into());
        }

        let outcome = self
            .call(contract.id(), "ft_transfer_call")
            .args_json(json_args)
            .max_gas()
            .deposit(NearToken::from_yoctonear(1))
            .transact()
            .await?
            .into_result()?;

        Ok(outcome.into())
    }

    async fn poa_force_sync_wrapped_token_metadata(
        &self,
        contract: &PoATokenContract,
        attached_deposit: NearToken,
    ) -> anyhow::Result<TestLog> {
        let outcome = self
            .call(contract.id(), "force_sync_wrapped_token_metadata")
            .max_gas()
            .deposit(attached_deposit)
            .transact()
            .await?
            .into_result()?;

        Ok(outcome.into())
    }

    async fn poa_lock_contract_for_wrapping(
        &self,
        contract_id: &AccountId,
    ) -> anyhow::Result<TestLog> {
        let outcome = self
            .call(contract_id, "lock_contract")
            .args_json(json!(
                {
                    "key": "ALL".to_string(),
                }
            ))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(outcome.into())
    }

    async fn poa_unlock_contract_for_wrapping(
        &self,
        contract: &AccountId,
    ) -> anyhow::Result<TestLog> {
        let outcome = self
            .call(contract, "unlock_contract")
            .args_json(json!(
                {
                    "key": "ALL".to_string(),
                }
            ))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(outcome.into())
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct TestLog {
    logs: Vec<String>,
    receipt_failure_errors: Vec<String>,
}

impl From<ExecutionResult<near_workspaces::result::Value>> for TestLog {
    fn from(outcome: ExecutionResult<near_workspaces::result::Value>) -> Self {
        Self {
            logs: outcome.logs().into_iter().map(str::to_string).collect(),
            receipt_failure_errors: outcome
                .receipt_outcomes()
                .iter()
                .map(|s| {
                    if let Err(e) = (*s).clone().into_result() {
                        match e.into_inner() {
                            Ok(o) => format!("OK: {o}"),
                            Err(e) => format!("Err: {e}"),
                        }
                    } else {
                        String::new()
                    }
                })
                .collect::<Vec<_>>(),
        }
    }
}

impl TestLog {
    pub fn logs(&self) -> &[String] {
        &self.logs
    }
}
