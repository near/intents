pub mod accounts;
mod env;
mod garbage_collector;
mod intents;
mod state;
mod storage;
mod tokens;
mod upgrade;

use self::accounts::AccountManagerExt;
use crate::utils::{account::AccountExt, crypto::Signer, read_wasm};
use arbitrary::{Arbitrary, Unstructured};
use defuse::core::payload::DefusePayload;
use defuse::core::sep53::Sep53Payload;
use defuse::core::ton_connect::tlb_ton::MsgAddress;
use defuse::{
    contract::config::DefuseConfig,
    core::{
        Deadline, Nonce,
        nep413::Nep413Payload,
        payload::{multi::MultiPayload, nep413::Nep413DefuseMessage},
        ton_connect::TonConnectPayload,
    },
};
use near_sdk::{AccountId, serde::Serialize, serde_json::json};
use near_sdk::{Gas, NearToken};
use near_workspaces::Contract;
use std::sync::LazyLock;

static DEFUSE_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("res/defuse"));
static DEFUSE_LEGACY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("releases/defuse-0.2.10.wasm"));

pub trait DefuseExt: AccountManagerExt {
    async fn deploy_defuse(
        &self,
        id: &str,
        config: DefuseConfig,
        legacy: bool,
    ) -> anyhow::Result<Contract>;

    async fn upgrade_defuse(&self, defuse_contract_id: &AccountId) -> anyhow::Result<()>;
}

impl DefuseExt for near_workspaces::Account {
    async fn deploy_defuse(
        &self,
        id: &str,
        config: DefuseConfig,
        legacy: bool,
    ) -> anyhow::Result<Contract> {
        let wasm = if legacy {
            &DEFUSE_LEGACY_WASM
        } else {
            &DEFUSE_WASM
        };

        let contract = self.deploy_contract(id, wasm).await?;

        contract
            .call("new")
            .args_json(json!({
                "config": config,
            }))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(contract)
    }

    async fn upgrade_defuse(&self, defuse_contract_id: &AccountId) -> anyhow::Result<()> {
        self.call(defuse_contract_id, "upgrade")
            .deposit(NearToken::from_yoctonear(1))
            .args_borsh((DEFUSE_WASM.clone(), None::<Gas>))
            .max_gas()
            .transact()
            .await?
            .into_result()?;

        Ok(())
    }
}

impl DefuseExt for Contract {
    async fn deploy_defuse(
        &self,
        id: &str,
        config: DefuseConfig,
        legacy: bool,
    ) -> anyhow::Result<Self> {
        self.as_account().deploy_defuse(id, config, legacy).await
    }

    async fn upgrade_defuse(&self, defuse_contract_id: &AccountId) -> anyhow::Result<()> {
        self.as_account().upgrade_defuse(defuse_contract_id).await
    }
}

pub trait DefuseSigner: Signer {
    #[must_use]
    fn sign_defuse_message<T>(
        &self,
        signing_standard: SigningStandard,
        defuse_contract: &AccountId,
        nonce: Nonce,
        deadline: Deadline,
        message: T,
    ) -> MultiPayload
    where
        T: Serialize;
}

impl DefuseSigner for near_workspaces::Account {
    fn sign_defuse_message<T>(
        &self,
        signing_standard: SigningStandard,
        defuse_contract: &AccountId,
        nonce: Nonce,
        deadline: Deadline,
        message: T,
    ) -> MultiPayload
    where
        T: Serialize,
    {
        match signing_standard {
            SigningStandard::Nep413 => self
                .sign_nep413(
                    Nep413Payload::new(
                        serde_json::to_string(&Nep413DefuseMessage {
                            signer_id: self.id().clone(),
                            deadline,
                            message,
                        })
                        .unwrap(),
                    )
                    .with_recipient(defuse_contract)
                    .with_nonce(nonce),
                )
                .into(),
            SigningStandard::TonConnect => self
                .sign_ton_connect(TonConnectPayload {
                    address: MsgAddress::arbitrary(&mut Unstructured::new(
                        self.secret_key().public_key().key_data(),
                    ))
                    .unwrap(),
                    domain: "intents.test.near".to_string(),
                    timestamp: defuse_near_utils::time::now(),
                    payload: defuse::core::ton_connect::TonConnectPayloadSchema::Text {
                        text: serde_json::to_string(&DefusePayload {
                            signer_id: self.id().clone(),
                            verifying_contract: defuse_contract.clone(),
                            deadline,
                            nonce,
                            message,
                        })
                        .unwrap(),
                    },
                })
                .into(),
            SigningStandard::Sep53 => self
                .sign_sep53(Sep53Payload::new(
                    serde_json::to_string(&DefusePayload {
                        signer_id: self.id().clone(),
                        verifying_contract: defuse_contract.clone(),
                        deadline,
                        nonce,
                        message,
                    })
                    .unwrap(),
                ))
                .into(),
            SigningStandard::Bip322 => self
                .sign_bip322(
                    serde_json::to_string(&DefusePayload {
                        signer_id: self.id().clone(),
                        verifying_contract: defuse_contract.clone(),
                        deadline,
                        nonce,
                        message,
                    })
                    .unwrap(),
                )
                .into(),
        }
    }
}

#[derive(Debug, Default, Arbitrary)]
pub enum SigningStandard {
    #[default]
    Nep413,
    TonConnect,
    Sep53,
    Bip322,
}
