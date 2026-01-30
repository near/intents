use defuse_sandbox::{SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountId, Gas, NearToken, serde_json::json};

use defuse::contract::config::DefuseConfig;

pub trait DefuseExt {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<SigningAccount>;

    async fn upgrade_defuse(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<()>;
}

impl DefuseExt for SigningAccount {
    async fn deploy_defuse(
        &self,
        name: impl AsRef<str>,
        config: DefuseConfig,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<Self> {
        self.deploy_sub_contract(
            name,
            NearToken::from_near(100),
            wasm,
            FnCallBuilder::new("new").json_args(json!({
                "config": config,
            })),
        )
        .await
    }

    async fn upgrade_defuse(
        &self,
        defuse_contract_id: impl Into<AccountId>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<()> {
        self.tx(defuse_contract_id)
            .function_call(
                FnCallBuilder::new("upgrade")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .borsh_args(&(&wasm.into(), None::<Gas>)),
            )
            .await?;

        Ok(())
    }
}
