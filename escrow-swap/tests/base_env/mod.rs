mod escrow;

pub use self::escrow::*;

use std::sync::LazyLock;

use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::FeesConfig,
    sandbox_ext::deployer::DefuseExt,
};
use defuse_fees::Pips;
use defuse_sandbox::{
    Account, Sandbox, SigningAccount, api::types::transaction::actions::GlobalContractDeployMode,
    extensions::wnear::WNearDeployerExt, read_wasm, sandbox,
};
use futures::try_join;
use impl_tools::autoimpl;
use near_sdk::{GlobalContractId, NearToken};
use rstest::fixture;

pub static ESCROW_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res", "defuse_escrow_swap"));

#[autoimpl(Deref using self.sandbox)]
pub struct BaseEnv {
    // pub wnear: Account,
    pub verifier: Account,
    pub escrow_global: GlobalContractId,

    sandbox: Sandbox,
}

#[fixture]
pub async fn env() -> BaseEnv {
    BaseEnv::new().await.unwrap()
}

impl BaseEnv {
    pub async fn new() -> anyhow::Result<Self> {
        let sandbox = sandbox(NearToken::from_near(10_000)).await;

        let wnear = sandbox.root().deploy_wrap_near("wnear").await?;
        let (verifier, escrow_global) = try_join!(
            // match len of intents.near
            sandbox.root().deploy_defuse(
                "vrfr",
                DefuseConfig {
                    wnear_id: wnear.id().clone(),
                    fees: FeesConfig {
                        fee: Pips::from_percent(1).unwrap(),
                        fee_collector: sandbox.root().id().clone(),
                    },
                    roles: RolesConfig::default(),
                },
                false
            ),
            sandbox.root().deploy_escrow_global("escrow"),
        )
        .unwrap();

        Ok(Self {
            // wnear,
            verifier,
            escrow_global,
            sandbox,
        })
    }
}

pub trait AccountExt {
    async fn deploy_escrow_global(&self, name: impl AsRef<str>)
    -> anyhow::Result<GlobalContractId>;
}

impl AccountExt for SigningAccount {
    async fn deploy_escrow_global(
        &self,
        name: impl AsRef<str>,
    ) -> anyhow::Result<GlobalContractId> {
        let account = self.sub_account(name)?;

        self.tx(account.clone())
            .create_account()
            .transfer(NearToken::from_near(100))
            .deploy_global(ESCROW_WASM.clone(), GlobalContractDeployMode::AccountId)
            .await?;

        Ok(GlobalContractId::AccountId(account))
    }
}
