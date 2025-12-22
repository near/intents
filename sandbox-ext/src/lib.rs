use std::{fs, path::Path, sync::LazyLock};

use defuse_escrow_proxy::{ProxyConfig, RolesConfig};
use defuse_sandbox::{FnCallBuilder, SigningAccount};
use near_sdk::{Gas, NearToken, serde_json::json};

#[track_caller]
fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/")
        .join(name)
        .with_extension("wasm");
    fs::read(filename.clone()).unwrap_or_else(|_| panic!("file {filename:?} should exists"))
}

pub static ESCROW_PROXY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("defuse_escrow_proxy"));

#[allow(async_fn_in_trait)]
pub trait EscrowProxyExt {
    async fn deploy_escrow_proxy(&self, roles: RolesConfig, config: ProxyConfig) -> anyhow::Result<()>;
    async fn get_escrow_proxy_config(&self) -> anyhow::Result<ProxyConfig>;
}

impl EscrowProxyExt for SigningAccount {
    async fn deploy_escrow_proxy(&self, roles: RolesConfig, config: ProxyConfig) -> anyhow::Result<()> {
        self.tx(self.id().clone())
            .transfer(NearToken::from_near(20))
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
}
