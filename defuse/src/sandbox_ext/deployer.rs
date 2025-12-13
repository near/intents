use std::sync::LazyLock;

use defuse_sandbox::{SigningAccount, anyhow, read_wasm, tx::FnCallBuilder};
use near_sdk::{AccountId, Gas, NearToken, serde_json::json};

use crate::contract::config::DefuseConfig;

static DEFUSE_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("res/defuse"));
static DEFUSE_LEGACY_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("releases/previous"));

pub trait DefuseExt {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        legacy: bool,
    ) -> anyhow::Result<SigningAccount>;

    async fn upgrade_defuse(&self, defuse_contract_id: impl Into<AccountId>) -> anyhow::Result<()>;
}

impl DefuseExt for SigningAccount {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        legacy: bool,
    ) -> anyhow::Result<SigningAccount> {
        self.deploy_sub_contract(
            name,
            NearToken::from_near(100),
            if legacy {
                DEFUSE_LEGACY_WASM.clone()
            } else {
                DEFUSE_WASM.clone()
            },
            FnCallBuilder::new("new").json_args(json!({
                "config": config,
            })),
        )
        .await
    }

    async fn upgrade_defuse(&self, defuse_contract_id: impl Into<AccountId>) -> anyhow::Result<()> {
        self.tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("upgrade")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .borsh_args(&(&DEFUSE_WASM.to_vec(), None::<Gas>)),
            )
            .await?;

        Ok(())
    }
}
