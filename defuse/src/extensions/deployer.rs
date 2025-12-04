use std::{fs, path::Path, sync::LazyLock};

use defuse_sandbox::{
    Account, SigningAccount, anyhow, extensions::account::AccountDeployerExt, tx::FnCallBuilder,
};
use near_sdk::{AccountIdRef, Gas, NearToken, serde_json::json};

#[track_caller]
pub(super) fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/")
        .join(name)
        .with_extension("wasm");
    fs::read(filename).unwrap()
}

static DEFUSE_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("res/defuse"));
static DEFUSE_LEGACY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("releases/defuse-0.2.10.wasm"));

use crate::contract::config::DefuseConfig;
pub trait DefuseExt: AccountDeployerExt {
    async fn deploy_defuse(
        &self,
        id: &str,
        config: DefuseConfig,
        legacy: bool,
    ) -> anyhow::Result<Account>;

    async fn upgrade_defuse(&self, defuse_contract_id: &AccountIdRef) -> anyhow::Result<()>;
}

impl DefuseExt for SigningAccount {
    async fn deploy_defuse(
        &self,
        id: &str,
        config: DefuseConfig,
        legacy: bool,
    ) -> anyhow::Result<Account> {
        let wasm = if legacy {
            &DEFUSE_LEGACY_WASM
        } else {
            &DEFUSE_WASM
        };

        let contract = self
            .deploy_contract(
                id,
                wasm.as_slice(),
                NearToken::from_near(100),
                Some(FnCallBuilder::new("new").json_args(&json!({
                    "config": config,
                }))),
            )
            .await?;

        Ok(contract)
    }

    async fn upgrade_defuse(&self, defuse_contract_id: &AccountIdRef) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.into())
            .function_call(
                FnCallBuilder::new("upgrade")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .borsh_args(&(DEFUSE_WASM.clone(), None::<Gas>)),
            )
            .await?;

        Ok(())
    }
}
