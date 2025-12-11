use std::{fs, path::Path, sync::LazyLock};

use crate::{ProxyConfig, RolesConfig};
use defuse_sandbox::{SigningAccount, TxResult};
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
pub trait EscrowProxyAccountExt {
    async fn deploy_escrow_proxy(&self, roles: RolesConfig, config: ProxyConfig) -> TxResult<()>;
    async fn get_escrow_proxy_config(&self) -> anyhow::Result<ProxyConfig>;
}

impl EscrowProxyAccountExt for SigningAccount {
    async fn deploy_escrow_proxy(&self, roles: RolesConfig, config: ProxyConfig) -> TxResult<()> {
        self.tx(self.id().clone())
            .transfer(NearToken::from_near(20))
            .deploy(ESCROW_PROXY_WASM.clone())
            .function_call_json::<()>(
                "new",
                json!({
                    "roles": roles,
                    "config": config,
                }),
                Gas::from_tgas(50),
                NearToken::from_yoctonear(0),
            )
            .no_result()
            .await?;

        Ok(())
    }

    async fn get_escrow_proxy_config(&self) -> anyhow::Result<ProxyConfig> {
        Ok(self
            .tx(self.id().clone())
            .function_call_json::<ProxyConfig>(
                "config",
                "{}",
                Gas::from_tgas(300),
                NearToken::from_near(0),
            )
            .await?)
    }
}
