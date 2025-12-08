use defuse_sandbox::{
    Account, SigningAccount, anyhow, extensions::account::AccountDeployerExt, tx::FnCallBuilder,
};
use near_sdk::{AccountIdRef, Gas, NearToken, serde_json::json};

use crate::contract::config::DefuseConfig;

// TODO: make it prettier
const DEFUSE_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../releases/defuse-0.4.0.wasm"
));

const DEFUSE_LEGACY_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../releases/defuse-0.2.10.wasm"
));

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
            DEFUSE_LEGACY_WASM
        } else {
            DEFUSE_WASM
        };

        self.deploy_contract(
            id,
            wasm,
            Some(FnCallBuilder::new("new").json_args(&json!({
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
                    .borsh_args(&(DEFUSE_WASM, None::<Gas>)),
            )
            .await?;

        Ok(())
    }
}
