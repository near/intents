use defuse::core::fees::Pips;
use near_sdk::{AccountId, NearToken};
use serde_json::json;

pub trait FeesManagerExt {
    async fn set_fee(&self, defuse_contract_id: &AccountId, fee: Pips) -> anyhow::Result<()>;

    async fn fee(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Pips>;
    async fn set_fee_collector(
        &self,
        defuse_contract_id: &AccountId,
        fee_collector: &AccountId,
    ) -> anyhow::Result<()>;
    async fn fee_collector(&self, defuse_contract_id: &AccountId) -> anyhow::Result<AccountId>;
}

impl FeesManagerExt for near_workspaces::Account {
    async fn set_fee(&self, defuse_contract_id: &AccountId, fee: Pips) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "set_fee")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "fee": fee,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
        Ok(())
    }

    async fn fee(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Pips> {
        self.view(defuse_contract_id, "fee")
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn set_fee_collector(
        &self,
        defuse_contract_id: &AccountId,
        fee_collector: &AccountId,
    ) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "set_fee_collector")
            .deposit(NearToken::from_yoctonear(1))
            .args_json(json!({
                "fee_collector": fee_collector,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }
    async fn fee_collector(&self, defuse_contract_id: &AccountId) -> anyhow::Result<AccountId> {
        self.view(defuse_contract_id, "fee_collector")
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl FeesManagerExt for near_workspaces::Contract {
    async fn set_fee(&self, defuse_contract_id: &AccountId, fee: Pips) -> anyhow::Result<()> {
        self.as_account().set_fee(defuse_contract_id, fee).await
    }

    async fn fee(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Pips> {
        self.as_account().fee(defuse_contract_id).await
    }

    async fn set_fee_collector(
        &self,
        defuse_contract_id: &AccountId,
        fee_collector: &AccountId,
    ) -> anyhow::Result<()> {
        self.as_account()
            .set_fee_collector(defuse_contract_id, fee_collector)
            .await
    }

    async fn fee_collector(&self, defuse_contract_id: &AccountId) -> anyhow::Result<AccountId> {
        self.as_account().fee_collector(defuse_contract_id).await
    }
}
