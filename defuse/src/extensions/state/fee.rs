use defuse_core::fees::Pips;
use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountId, NearToken, serde_json::json};

pub trait FeesManagerExt {
    async fn set_fee(&self, defuse_contract_id: &AccountId, fee: Pips) -> anyhow::Result<()>;

    async fn set_fee_collector(
        &self,
        defuse_contract_id: &AccountId,
        fee_collector: &AccountId,
    ) -> anyhow::Result<()>;
}

pub trait FeesManagerViewExt {
    async fn fee(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Pips>;

    async fn fee_collector(&self, defuse_contract_id: &AccountId) -> anyhow::Result<AccountId>;
}

impl FeesManagerExt for SigningAccount {
    async fn set_fee(&self, defuse_contract_id: &AccountId, fee: Pips) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.clone())
            .function_call(
                FnCallBuilder::new("set_fee")
                    .json_args(&json!({
                        "fee": fee,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .into_result()?;

        Ok(())
    }

    async fn set_fee_collector(
        &self,
        defuse_contract_id: &AccountId,
        fee_collector: &AccountId,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.clone())
            .function_call(
                FnCallBuilder::new("set_fee_collector")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(&json!({
                        "fee_collector": fee_collector,
                    })),
            )
            .await?
            .into_result()?;

        Ok(())
    }
}

impl FeesManagerViewExt for Account {
    async fn fee(&self, defuse_contract_id: &AccountId) -> anyhow::Result<Pips> {
        let account = Account::new(defuse_contract_id.clone(), self.network_config().clone());

        account
            .call_view_function_json("fee", ())
            .await
            .map_err(Into::into)
    }

    async fn fee_collector(&self, defuse_contract_id: &AccountId) -> anyhow::Result<AccountId> {
        let account = Account::new(defuse_contract_id.clone(), self.network_config().clone());

        account
            .call_view_function_json("fee_collector", ())
            .await
            .map_err(Into::into)
    }
}
