use std::sync::LazyLock;

use near_sdk::{AccountId, NearToken, serde_json::json};

use super::ft::FtExt;
use crate::{SigningAccount, read_wasm, tx::FnCallBuilder};

static WNEAR_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("releases/wnear"));

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
    async fn deploy_wrap_near(&self, name: impl AsRef<str>) -> anyhow::Result<SigningAccount>;
}

impl WNearDeployerExt for SigningAccount {
    async fn deploy_wrap_near(&self, name: impl AsRef<str>) -> anyhow::Result<Self> {
        self.deploy_sub_contract(
            name,
            NearToken::from_near(100),
            WNEAR_WASM.clone(),
            FnCallBuilder::new("new"),
        )
        .await
    }
}
