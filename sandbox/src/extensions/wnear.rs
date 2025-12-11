use std::sync::LazyLock;

use near_sdk::{AccountIdRef, NearToken, serde_json::json};

use super::ft::FtExt;
use crate::{Account, SigningAccount, read_wasm, tx::FnCallBuilder};

static WNEAR_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("releases/wnear.wasm"));

#[allow(async_fn_in_trait)]
pub trait WNearDeployerExt {
    async fn deploy_wrap_near(&self, token: &str) -> anyhow::Result<Account>;
}

#[allow(async_fn_in_trait)]
pub trait WNearExt: FtExt {
    async fn near_deposit(&self, wnear_id: &AccountIdRef, amount: NearToken) -> anyhow::Result<()>;
    async fn near_withdraw(&self, wnear_id: &AccountIdRef, amount: NearToken)
    -> anyhow::Result<()>;
}

impl WNearDeployerExt for SigningAccount {
    async fn deploy_wrap_near(&self, token: &str) -> anyhow::Result<Account> {
        let contract = self
            .deploy_contract(token, WNEAR_WASM.to_vec(), Some(FnCallBuilder::new("new")))
            .await?;

        Ok(contract)
    }
}

impl WNearExt for SigningAccount {
    async fn near_deposit(&self, wnear_id: &AccountIdRef, amount: NearToken) -> anyhow::Result<()> {
        self.tx(wnear_id.into())
            .function_call(
                FnCallBuilder::new("near_deposit")
                    .with_deposit(NearToken::from_yoctonear(amount.as_yoctonear())),
            )
            .await?;

        Ok(())
    }

    async fn near_withdraw(
        &self,
        wnear_id: &AccountIdRef,
        amount: NearToken,
    ) -> anyhow::Result<()> {
        self.tx(wnear_id.into())
            .function_call(FnCallBuilder::new("near_withdraw").json_args(json!({
                "amount": amount,
            })))
            .await?;

        Ok(())
    }
}
