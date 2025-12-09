use std::sync::LazyLock;

use defuse_sandbox::{
    Account, SigningAccount, anyhow, extensions::account::AccountDeployerExt, read_wasm,
    tx::FnCallBuilder,
};
use near_sdk::{AccountIdRef, Gas, NearToken, serde_json::json};

use crate::contract::config::DefuseConfig;

static DEFUSE_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("releases/defuse-0.4.0.wasm"));
static DEFUSE_LEGACY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("releases/defuse-0.2.10.wasm"));

#[allow(async_fn_in_trait)]
pub trait DefuseExt: AccountDeployerExt {
    async fn deploy_defuse(
        &self,
        id: impl AsRef<str>,
        config: DefuseConfig,
        legacy: bool,
    ) -> anyhow::Result<Account>;

    async fn upgrade_defuse(&self, defuse_contract_id: &AccountIdRef) -> anyhow::Result<()>;
}

impl DefuseExt for SigningAccount {
    async fn deploy_defuse(
        &self,
        id: impl AsRef<str>,
        config: DefuseConfig,
        legacy: bool,
    ) -> anyhow::Result<Account> {
        let wasm = if legacy {
            &DEFUSE_LEGACY_WASM
        } else {
            &DEFUSE_WASM
        };

        self.deploy_contract(
            id,
            wasm.to_vec(),
            Some(FnCallBuilder::new("new").json_args(json!({
                "config": config,
            }))),
        )
        .await
    }

    async fn upgrade_defuse(&self, defuse_contract_id: &AccountIdRef) -> anyhow::Result<()> {
        self.tx(defuse_contract_id.into())
            .function_call(
                FnCallBuilder::new("upgrade")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .borsh_args(&(&DEFUSE_WASM.to_vec(), None::<Gas>)),
            )
            .await?;

        Ok(())
    }
}
