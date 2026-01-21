use std::{fs, path::Path, sync::LazyLock};

use defuse_escrow_proxy::ProxyConfig;
#[cfg(feature = "escrow-swap")]
use defuse_escrow_swap::Params as EscrowParams;
#[cfg(feature = "escrow-swap")]
use near_sdk::AccountId;
use near_sdk::{Gas, NearToken};
use serde_json::json;

use crate::{FnCallBuilder, SigningAccount};

pub static ESCROW_PROXY_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR")).join("../res/defuse_escrow_proxy.wasm");
    fs::read(filename.clone()).unwrap_or_else(|_| panic!("file {filename:?} should exist"))
});

pub trait EscrowProxyExt {
    async fn deploy_escrow_proxy(&self, config: ProxyConfig) -> anyhow::Result<()>;
    async fn get_escrow_proxy_config(&self) -> anyhow::Result<ProxyConfig>;
    /// Call `cancel_escrow` on proxy contract. Requires caller to be owner.
    #[cfg(feature = "escrow-swap")]
    async fn cancel_escrow(
        &self,
        proxy_contract: &AccountId,
        params: &EscrowParams,
    ) -> anyhow::Result<()>;
}

impl EscrowProxyExt for SigningAccount {
    async fn deploy_escrow_proxy(&self, config: ProxyConfig) -> anyhow::Result<()> {
        self.tx(self.id().clone())
            .transfer(NearToken::from_near(5))
            .deploy(ESCROW_PROXY_WASM.clone())
            .function_call(
                FnCallBuilder::new("new")
                    .json_args(json!({
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

    #[cfg(feature = "escrow-swap")]
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
