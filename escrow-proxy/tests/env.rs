use std::{fs, path::Path, sync::LazyLock};

use defuse_escrow_proxy::{ProxyConfig, RolesConfig};
use defuse_sandbox::{Account, Sandbox, SigningAccount, TxResult};
use defuse_transfer_auth::ext::TransferAuthAccountExt;
use impl_tools::autoimpl;
use near_sdk::{AccountId, Gas, NearToken, serde_json::json};

#[track_caller]
fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/")
        .join(name)
        .with_extension("wasm");
    fs::read(filename.clone()).expect(&format!("file {filename:?} should exists"))
}

pub static ESCROW_PROXY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("defuse_escrow_proxy"));

#[autoimpl(Deref using self.sandbox)]
pub struct BaseEnv {
    // pub verifier: Account,
    // pub transfer_auth_global: AccountId,
    //
    sandbox: Sandbox,
}

impl BaseEnv {
    pub async fn new() -> TxResult<Self> {
        let sandbox = Sandbox::new().await;

        // let (verifier, transfer_auth_global) = futures::join!(
        //     // match len of intents.near
        //     sandbox.root().deploy_verifier("vrfr", wnear.id().clone()),
        //     sandbox.root().deploy_transfer_auth("auth"),
        // );
        //

        Ok(Self {
            sandbox,
        })
    }

    pub fn root(&self) -> &SigningAccount {
        self.sandbox.root()
    }
}

pub trait AccountExt {
    async fn deploy_escrow_proxy(&self, roles: RolesConfig, config: ProxyConfig) -> TxResult<()>;
    async fn get_escrow_proxy_config(
        &self,
    ) -> anyhow::Result<ProxyConfig>;
}

impl AccountExt for SigningAccount {
    async fn deploy_escrow_proxy(&self, roles: RolesConfig, config: ProxyConfig) -> TxResult<()>{
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

    async fn get_escrow_proxy_config(
        &self,
    ) -> anyhow::Result<ProxyConfig> {
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
