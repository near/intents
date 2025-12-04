use defuse_core::fees::Pips;
use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountId, AccountIdRef, NearToken, serde_json::json};

pub trait FeesManagerExt {
    async fn set_fee(&self, defuse_contract_id: &AccountIdRef, fee: Pips) -> anyhow::Result<()>;

    async fn set_fee_collector(
        &self,
        defuse_contract_id: &AccountIdRef,
        fee_collector: &AccountIdRef,
    ) -> anyhow::Result<()>;
}

pub trait FeesManagerViewExt {
    async fn fee(&self) -> anyhow::Result<Pips>;

    async fn fee_collector(&self) -> anyhow::Result<AccountId>;
}

impl FeesManagerExt for SigningAccount {
    async fn set_fee(&self, defuse_contract_id: &AccountIdRef, fee: Pips) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.into())
            .function_call(
                FnCallBuilder::new("set_fee")
                    .json_args(&json!({
                        "fee": fee,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(())
    }

    async fn set_fee_collector(
        &self,
        defuse_contract_id: &AccountIdRef,
        fee_collector: &AccountIdRef,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.into())
            .function_call(
                FnCallBuilder::new("set_fee_collector")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(&json!({
                        "fee_collector": fee_collector,
                    })),
            )
            .await?;

        Ok(())
    }
}

impl FeesManagerViewExt for Account {
    async fn fee(&self) -> anyhow::Result<Pips> {
        self.call_view_function_json("fee", ())
            .await
            .map_err(Into::into)
    }

    async fn fee_collector(&self) -> anyhow::Result<AccountId> {
        self.call_view_function_json("fee_collector", ())
            .await
            .map_err(Into::into)
    }
}

impl FeesManagerViewExt for SigningAccount {
    async fn fee(&self) -> anyhow::Result<Pips> {
        self.account().fee().await
    }

    async fn fee_collector(&self) -> anyhow::Result<AccountId> {
        self.account().fee_collector().await
    }
}
