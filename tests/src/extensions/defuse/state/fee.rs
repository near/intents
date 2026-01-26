use defuse::core::fees::Pips;
use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountId, AccountIdRef, NearToken, serde_json::json};

#[allow(async_fn_in_trait)]
pub trait FeesManagerExt {
    async fn set_fee(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        fee: Pips,
    ) -> anyhow::Result<()>;

    async fn set_fee_collector(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        fee_collector: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()>;
}

#[allow(async_fn_in_trait)]
pub trait FeesManagerViewExt {
    async fn fee(&self) -> anyhow::Result<Pips>;

    async fn fee_collector(&self) -> anyhow::Result<AccountId>;
}

impl FeesManagerExt for SigningAccount {
    async fn set_fee(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        fee: Pips,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("set_fee")
                    .json_args(json!({
                        "fee": fee,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(())
    }

    async fn set_fee_collector(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        fee_collector: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("set_fee_collector")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "fee_collector": fee_collector.as_ref(),
                    })),
            )
            .await?;

        Ok(())
    }
}

impl FeesManagerViewExt for Account {
    async fn fee(&self) -> anyhow::Result<Pips> {
        self.call_view_function_json("fee", ()).await
    }

    async fn fee_collector(&self) -> anyhow::Result<AccountId> {
        self.call_view_function_json("fee_collector", ()).await
    }
}
