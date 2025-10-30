mod mt;
mod sandbox;
mod utils;

pub use self::utils::*;

use std::sync::LazyLock;

use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::FeesConfig,
};
use defuse_escrow::{FixedParams, Params, Storage};
use defuse_fees::Pips;
use futures::join;
use impl_tools::autoimpl;
use near_api::types::transaction::actions::{GlobalContractDeployMode, GlobalContractIdentifier};
use near_sdk::{AccountId, Gas, NearToken, serde_json::json};

use crate::env::{sandbox::Sandbox, utils::read_wasm};

pub static WNEAR_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("../tests/contracts/wnear"));
pub static VERIFIER_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse"));
pub static ESCROW_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse_escrow"));

#[autoimpl(Deref using self.sandbox)]
pub struct Env {
    pub wnear: Account,
    pub verifier: Account,
    pub escrow_global: GlobalContractIdentifier,

    sandbox: Sandbox,
}

impl Env {
    pub async fn new() -> Self {
        let sandbox = Sandbox::new().await;

        let wnear = sandbox.deploy_wnear("wnear").await;
        let (verifier, escrow_global) = join!(
            // match len of intents.near
            sandbox.deploy_verifier("vrfr", wnear.id().clone()),
            sandbox.deploy_escrow_global("escrow"),
        );

        Env {
            wnear,
            verifier,
            escrow_global,
            sandbox,
        }
    }

    pub async fn create_escrow(&self, fixed: &FixedParams, params: Params) -> Account {
        let init_args = json!({
            "fixed": fixed,
            "params": params,
        });

        let account_id = Storage::new(fixed, params).derive_account_id(self.id());

        self.tx(account_id.clone())
            .create_account()
            .use_global(self.escrow_global.clone())
            .function_call_json(
                "new",
                init_args,
                Gas::from_tgas(10),
                NearToken::from_yoctonear(0),
            )
            .await
            .unwrap()
            .into_result()
            .inspect(|r| println!("create escrow {account_id}::new(): {:#?}", r.logs()))
            .unwrap();

        Account::new(account_id, self.network_config().clone())
    }
}

impl SigningAccount {
    async fn deploy_wnear(&self, name: impl AsRef<str>) -> Account {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(20))
            .deploy(WNEAR_WASM.clone())
            .function_call_json("new", (), Gas::from_tgas(50), NearToken::from_yoctonear(0))
            .await
            .unwrap()
            .into_result()
            .unwrap();

        account
    }

    async fn deploy_verifier(&self, name: impl AsRef<str>, wnear_id: AccountId) -> Account {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(20))
            .deploy(VERIFIER_WASM.clone())
            .function_call_json(
                "new",
                json!({
                    "config": DefuseConfig {
                        wnear_id,
                        fees: FeesConfig {
                            fee: Pips::from_percent(1).unwrap(),
                            fee_collector: self.id().clone()
                        },
                        roles: RolesConfig::default()
                    }
                }),
                Gas::from_tgas(50),
                NearToken::from_yoctonear(0),
            )
            .await
            .unwrap()
            .into_result()
            .unwrap();

        account
    }

    async fn deploy_escrow_global(&self, name: impl AsRef<str>) -> GlobalContractIdentifier {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(100))
            .deploy_global(ESCROW_WASM.clone(), GlobalContractDeployMode::AccountId)
            .await
            .unwrap()
            .into_result()
            .unwrap();

        GlobalContractIdentifier::AccountId(account.id().clone())
    }
}
