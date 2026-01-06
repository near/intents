use std::{fs, path::Path, sync::LazyLock};

use defuse_escrow_proxy::{EscrowParams, ProxyConfig, RolesConfig};
use defuse_sandbox::{FnCallBuilder, SigningAccount};
use near_sdk::{AccountId, Gas, NearToken};
use serde_json::json;

pub static ESCROW_PROXY_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/defuse_escrow_proxy.wasm");
    fs::read(filename.clone()).unwrap_or_else(|_| panic!("file {filename:?} should exists"))
});

#[allow(async_fn_in_trait)]
pub trait EscrowProxyExt {
    async fn deploy_escrow_proxy(&self, roles: RolesConfig, config: ProxyConfig) -> anyhow::Result<()>;
    async fn get_escrow_proxy_config(&self) -> anyhow::Result<ProxyConfig>;
    /// Call cancel_escrow on proxy contract. Requires caller to have DAO or Canceller role.
    async fn cancel_escrow(
        &self,
        proxy_contract: &AccountId,
        params: &EscrowParams,
    ) -> anyhow::Result<()>;
}

impl EscrowProxyExt for SigningAccount {
    async fn deploy_escrow_proxy(&self, roles: RolesConfig, config: ProxyConfig) -> anyhow::Result<()> {
        self.tx(self.id().clone())
            .transfer(NearToken::from_near(5))
            .deploy(ESCROW_PROXY_WASM.clone())
            .function_call(
                FnCallBuilder::new("new")
                    .json_args(json!({
                        "roles": roles,
                        "config": config,
                    }))
                    .with_gas(Gas::from_tgas(50)),
            )
            .await?;

        Ok(())
    }

    async fn get_escrow_proxy_config(&self) -> anyhow::Result<ProxyConfig> {
        self.call_view_function_json("config", json!({})).await
    }

    async fn cancel_escrow(
        &self,
        proxy_contract: &AccountId,
        params: &EscrowParams,
    ) -> anyhow::Result<()> {
        self.tx(proxy_contract.clone())
            .function_call(
                FnCallBuilder::new("cancel_escrow")
                    .json_args(json!({
                        "params": params,
                    }))
                    .with_gas(Gas::from_tgas(100))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;
        Ok(())
    }
}
