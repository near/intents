use std::{fs, path::Path, sync::LazyLock};

use defuse_fees::Pips;
use defuse_sandbox::{
    Account, Sandbox, SigningAccount, TxResult,
    api::types::transaction::actions::GlobalContractDeployMode,
};
use futures::join;
use impl_tools::autoimpl;
use near_sdk::{AccountId, Gas, GlobalContractId, NearToken, serde_json::json};

#[track_caller]
fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/")
        .join(name)
        .with_extension("wasm");
    fs::read(filename.clone()).expect(&format!("file {filename:?} should exists"))
}

pub static WNEAR_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("../tests/contracts/target/wnear"));
pub static VERIFIER_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse"));
pub static TRANSFER_AUTH_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse_transfer_auth"));

#[autoimpl(Deref using self.sandbox)]
pub struct BaseEnv {
    // pub wnear: Account,
    pub verifier: Account,
    pub transfer_auth_global: AccountId,

    sandbox: Sandbox,
}

impl BaseEnv {
    pub async fn new() -> TxResult<Self> {
        let sandbox = Sandbox::new().await;

        let wnear = sandbox.root().deploy_wnear("wnear").await;
        let (verifier, transfer_auth_global) = join!(
            // match len of intents.near
            sandbox.root().deploy_verifier("vrfr", wnear.id().clone()),
            sandbox.root().deploy_transfer_auth("auth"),
        );

        Ok(Self {
            // wnear,
            verifier,
            transfer_auth_global,
            sandbox,
        })
    }

    // pub async fn create_escrow(&self, params: &Params) -> TxResult<Account> {
    //     self.root()
    //         .deploy_escrow(self.escrow_global.clone(), params)
    //         .await
    // }
}

pub trait AccountExt {
    async fn deploy_wnear(&self, name: impl AsRef<str>) -> Account;
    async fn deploy_verifier(&self, name: impl AsRef<str>, wnear_id: AccountId) -> Account;
    async fn deploy_transfer_auth(&self, name: impl AsRef<str>) -> AccountId;
}

impl AccountExt for SigningAccount {
    async fn deploy_wnear(&self, name: impl AsRef<str>) -> Account {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(20))
            .deploy(WNEAR_WASM.clone())
            .function_call_json::<()>("new", (), Gas::from_tgas(50), NearToken::from_yoctonear(0))
            .no_result()
            .await
            .unwrap();

        account
    }

    async fn deploy_verifier(&self, name: impl AsRef<str>, wnear_id: AccountId) -> Account {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(20))
            .deploy(VERIFIER_WASM.clone())
            .function_call_json::<()>(
                "new",
                json!({
                    "config": json!({
                        "wnear_id": wnear_id,
                        "fees": {
                            "fee": Pips::from_percent(1).unwrap(),
                            "fee_collector": self.id().clone(),
                        },
                    }),
                }),
                Gas::from_tgas(50),
                NearToken::from_yoctonear(0),
            )
            .no_result()
            .await
            .unwrap();

        account
    }

    async fn deploy_transfer_auth(&self, name: impl AsRef<str>) -> AccountId {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(100))
            .deploy_global(TRANSFER_AUTH_WASM.clone(), GlobalContractDeployMode::AccountId)
            .await
            .unwrap();

        account.id().clone()
    }
}
