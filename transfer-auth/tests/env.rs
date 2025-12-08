use std::{fs, path::Path, sync::LazyLock};

use defuse_fees::Pips;
use defuse_sandbox::{
    Account, Sandbox, SigningAccount, TxResult,
    api::types::transaction::actions::GlobalContractDeployMode,
};
use defuse_transfer_auth::storage::{ContractStorage, State};
use futures::join;
use impl_tools::autoimpl;
use near_sdk::{
    AccountId, Gas, GlobalContractId, NearToken,
    serde_json::json,
    state_init::{StateInit, StateInitV1},
};

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
pub static TRANSFER_AUTH_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("defuse_transfer_auth"));

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

    pub async fn account_exists(&self, account_id: AccountId) -> bool {
        Account::new(account_id, self.sandbox.root().network_config().clone()).exists().await
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
    async fn deploy_transfer_auth_instance(
        &self,
        global_contract_id: AccountId,
        state: State,
    ) -> AccountId;
    async fn get_transfer_auth_instance_state(
        &self,
        global_contract_id: AccountId,
    ) -> anyhow::Result<ContractStorage>;
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
            .deploy_global(
                TRANSFER_AUTH_WASM.clone(),
                GlobalContractDeployMode::AccountId,
            )
            .await
            .unwrap();

        account.id().clone()
    }

    async fn deploy_transfer_auth_instance(
        &self,
        global_contract_id: AccountId,
        state: State,
    ) -> AccountId {
        let raw_state = ContractStorage::init_state(state.clone()).unwrap();
        let solver1_state_init = StateInit::V1(StateInitV1 {
            code: near_sdk::GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state.clone(),
        });

        let account = solver1_state_init.derive_account_id();

        //NOTE: there is rpc error on state_init action but the contract itself is successfully
        //deployed, so lets ignore error for now
        let _ = self.tx(account.clone())
            .state_init(global_contract_id, raw_state)
            .transfer(NearToken::from_yoctonear(1))
            .await;
        account
    }

    async fn get_transfer_auth_instance_state(
        &self,
        global_contract_id: AccountId,
    ) -> anyhow::Result<ContractStorage> {
    Ok(self
        .tx(global_contract_id)
        .function_call_json::<ContractStorage>(
            "state",
            "{}",
            Gas::from_tgas(300),
            NearToken::from_near(0),
        )
        .await?)


    }

}
