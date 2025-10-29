use std::sync::{Arc, LazyLock};

use defuse::{
    contract::config::{DefuseConfig, RolesConfig},
    core::fees::FeesConfig,
};
use defuse_escrow::State;
use defuse_fees::Pips;
use futures::join;
use impl_tools::autoimpl;
use near_api::{
    Account, Contract, Signer, Transaction,
    signer::generate_secret_key,
    types::{
        Action,
        transaction::actions::{
            CreateAccountAction, DeployContractAction, DeployGlobalContractAction,
            FunctionCallAction, GlobalContractDeployMode, GlobalContractIdentifier, TransferAction,
            UseGlobalContractAction,
        },
    },
};
use near_sdk::{
    AccountId, Gas, NearToken, borsh, env,
    serde_json::{self, json},
};

use crate::env::{sandbox::Sandbox, utils::read_wasm};

mod sandbox;
mod utils;

pub static WNEAR_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("../tests/contracts/wnear"));
pub static VERIFIER_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse"));
pub static ESCROW_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("defuse_escrow"));

#[autoimpl(Deref using self.sandbox)]
pub struct Env {
    pub wnear: Contract,
    pub verifier: Contract,
    pub escrow_global_id: AccountId,

    sandbox: Sandbox,
}

impl Env {
    pub async fn new() -> Self {
        let sandbox = Sandbox::new().await;

        let env = Env {
            wnear: Contract(sandbox.subaccount("wnear")),
            // match len of intents.near
            verifier: Contract(sandbox.subaccount("vrfr")),
            escrow_global_id: sandbox.subaccount("escrow"),
            sandbox,
        };

        join!(
            env.deploy_wnear(),
            env.deploy_verifier(),
            env.deploy_escrow_global()
        );

        env
    }

    fn subaccount(&self, name: impl AsRef<str>) -> AccountId {
        format!("{}.{}", name.as_ref(), self.root_id())
            .parse()
            .unwrap()
    }

    async fn deploy_wnear(&self) {
        Transaction::construct(self.root_id().clone(), self.wnear.0.clone())
            .add_action(Action::CreateAccount(CreateAccountAction {}))
            .add_action(Action::Transfer(TransferAction {
                deposit: NearToken::from_near(20),
            }))
            .add_action(Action::DeployContract(DeployContractAction {
                code: WNEAR_WASM.clone(),
            }))
            .add_action(Action::FunctionCall(
                FunctionCallAction {
                    method_name: "new".to_string(),
                    args: vec![],
                    gas: Gas::from_tgas(50),
                    deposit: NearToken::from_yoctonear(0),
                }
                .into(),
            ))
            .with_signer(self.root_signer())
            .send_to(self.network_config())
            .await
            .unwrap()
            .into_result()
            .unwrap();
    }

    async fn deploy_verifier(&self) {
        Transaction::construct(self.root_id().clone(), self.verifier.0.clone())
            .add_action(Action::CreateAccount(CreateAccountAction {}))
            .add_action(Action::Transfer(TransferAction {
                deposit: NearToken::from_near(20),
            }))
            .add_action(Action::DeployContract(DeployContractAction {
                code: VERIFIER_WASM.clone(),
            }))
            .add_action(Action::FunctionCall(
                FunctionCallAction {
                    method_name: "new".to_string(),
                    args: serde_json::to_vec(&json!({
                        "config": DefuseConfig {
                            wnear_id: self.wnear.0.clone(),
                            fees: FeesConfig {
                                fee: Pips::from_percent(1).unwrap(),
                                fee_collector: self.root_id().clone()
                            },
                            roles: RolesConfig::default()
                        }
                    }))
                    .unwrap(),
                    gas: Gas::from_tgas(50),
                    deposit: NearToken::from_yoctonear(0),
                }
                .into(),
            ))
            .with_signer(self.root_signer())
            .send_to(self.network_config())
            .await
            .unwrap()
            .into_result()
            .unwrap();
    }

    async fn deploy_escrow_global(&self) {
        // deploy escrow as global contract
        Transaction::construct(self.root_id().clone(), self.escrow_global_id.clone())
            .add_action(Action::CreateAccount(CreateAccountAction {}))
            .add_action(Action::Transfer(TransferAction {
                deposit: NearToken::from_near(100),
            }))
            .add_action(Action::DeployGlobalContract(DeployGlobalContractAction {
                code: ESCROW_WASM.clone(),
                deploy_mode: GlobalContractDeployMode::AccountId,
            }))
            .with_signer(self.root_signer())
            .send_to(self.network_config())
            .await
            .unwrap()
            .into_result()
            .unwrap();
    }

    pub async fn create_subaccount(
        &self,
        name: impl AsRef<str>,
        balance: NearToken,
    ) -> (AccountId, Arc<Signer>) {
        let account_id = self.subaccount(name);

        let secret_key = generate_secret_key().unwrap();
        let public_key = secret_key.public_key();
        let signer = Signer::new(Signer::from_secret_key(secret_key)).unwrap();

        Account::create_account(account_id.clone())
            .fund_myself(self.root_id().clone(), balance)
            .public_key(public_key)
            .unwrap()
            .with_signer(self.root_signer())
            .send_to(self.network_config())
            .await
            .unwrap()
            .into_result()
            .unwrap();

        (account_id, signer)
    }

    pub async fn create_escrow(&self, params: State) -> Contract {
        let escrow_id = {
            let serialized = borsh::to_vec(&params).unwrap();
            println!("serialized: {} bytes", serialized.len());
            self.subaccount(hex::encode(
                &env::keccak256_array(&serialized)
                    [32 - (AccountId::MAX_LEN - self.root_id().len() - 1).div_ceil(2)..32],
            ))
        };

        Transaction::construct(self.sandbox.root_id().clone(), escrow_id.clone())
            .add_action(Action::CreateAccount(CreateAccountAction {}))
            .add_action(Action::UseGlobalContract(
                UseGlobalContractAction {
                    contract_identifier: GlobalContractIdentifier::AccountId(
                        self.escrow_global_id.clone(),
                    ),
                }
                .into(),
            ))
            .add_action(Action::FunctionCall(
                FunctionCallAction {
                    method_name: "new".to_string(),
                    args: serde_json::to_vec(&json!({
                        "params": params,
                    }))
                    .unwrap(),
                    gas: Gas::from_tgas(10),
                    deposit: NearToken::from_yoctonear(0),
                }
                .into(),
            ))
            .with_signer(self.root_signer())
            .send_to(self.network_config())
            .await
            .unwrap()
            .into_result()
            .unwrap();

        Contract(escrow_id)
    }
}
