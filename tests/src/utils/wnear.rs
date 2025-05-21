use near_sdk::{AccountId, NearToken};
use near_workspaces::{Account, Contract};
use serde_json::json;

use super::{account::AccountExt, ft::FtExt};

const WNEAR_WASM: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/contracts/wnear.wasm"));

pub trait WNearExt: FtExt {
    async fn deploy_wrap_near(&self, token: &str) -> anyhow::Result<Contract>;
    async fn near_deposit(&self, wnear_id: &AccountId, amount: NearToken) -> anyhow::Result<()>;
    async fn near_withdraw(&self, wnear_id: &AccountId, amount: NearToken) -> anyhow::Result<()>;
}

impl WNearExt for Account {
    async fn deploy_wrap_near(&self, token: &str) -> anyhow::Result<Contract> {
        let contract = self.deploy_contract(token, WNEAR_WASM).await?;
        contract
            .call("new")
            .args_json(json!({}))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(contract)
    }

    async fn near_deposit(&self, wnear_id: &AccountId, amount: NearToken) -> anyhow::Result<()> {
        self.call(wnear_id, "near_deposit")
            .deposit(amount)
            .args_json(json!({}))
            .transact()
            .await?
            .into_result()
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn near_withdraw(&self, wnear_id: &AccountId, amount: NearToken) -> anyhow::Result<()> {
        self.call(wnear_id, "near_withdraw")
            .args_json(json!({
                "amount": amount,
            }))
            .transact()
            .await?
            .into_result()
            .map(|_| ())
            .map_err(Into::into)
    }
}

impl WNearExt for Contract {
    async fn deploy_wrap_near(&self, token: &str) -> anyhow::Result<Self> {
        self.as_account().deploy_wrap_near(token).await
    }

    async fn near_deposit(&self, wnear_id: &AccountId, amount: NearToken) -> anyhow::Result<()> {
        self.as_account().near_deposit(wnear_id, amount).await
    }

    async fn near_withdraw(&self, wnear_id: &AccountId, amount: NearToken) -> anyhow::Result<()> {
        self.as_account().near_withdraw(wnear_id, amount).await
    }
}
