use near_sdk::{AccountId, NearToken, serde_json::json};

use super::ft::FtExt;
use crate::{SigningAccount, tx::FnCallBuilder};

pub trait WNearExt: FtExt {
    async fn near_deposit(
        &self,
        wnear_id: impl Into<AccountId>,
        amount: NearToken,
    ) -> anyhow::Result<()>;
    async fn near_withdraw(
        &self,
        wnear_id: impl Into<AccountId>,
        amount: NearToken,
    ) -> anyhow::Result<()>;
}

impl WNearExt for SigningAccount {
    async fn near_deposit(
        &self,
        wnear_id: impl Into<AccountId>,
        amount: NearToken,
    ) -> anyhow::Result<()> {
        self.tx(wnear_id)
            .function_call(FnCallBuilder::new("near_deposit").with_deposit(amount))
            .await?;

        Ok(())
    }

    async fn near_withdraw(
        &self,
        wnear_id: impl Into<AccountId>,
        amount: NearToken,
    ) -> anyhow::Result<()> {
        self.tx(wnear_id)
            .function_call(
                FnCallBuilder::new("near_withdraw")
                    .json_args(json!({
                        "amount": amount,
                    }))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(())
    }
}

pub trait WNearDeployerExt {
    async fn deploy_wrap_near(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<SigningAccount>;
}

impl WNearDeployerExt for SigningAccount {
    async fn deploy_wrap_near(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<Self> {
        self.deploy_sub_contract(
            name,
            NearToken::from_near(100),
            wasm.into(),
            FnCallBuilder::new("new"),
        )
        .await
    }
}
