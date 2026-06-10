use anyhow::Result;
use defuse_outlayer_app::{AsHex, State as OutlayerState};
use near_kit::{GlobalContractIdentifier, Near};
use near_sdk::{
    AccountId, NearToken,
    serde::{Deserialize, Serialize},
};

use crate::{nep616::DeployDeterministicAccountExt, outcome::SuccessfulExecutionOutcome};

pub use defuse_outlayer_app as contract;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct OaSetCodeArgs {
    pub old_code_hash: AsHex<[u8; 32]>,
    pub new_code_hash: AsHex<[u8; 32]>,
    pub new_code_url: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct OaTransferAdminArgs {
    pub new_admin_id: AccountId,
}

#[near_kit::contract]
pub trait OutlayerApp {
    #[call]
    fn oa_set_code(&mut self, args: OaSetCodeArgs);

    #[call]
    fn oa_transfer_admin(&mut self, args: OaTransferAdminArgs);

    fn oa_admin_id(&self) -> AccountId;
    fn oa_code_hash(&self) -> AsHex<[u8; 32]>;
    fn oa_code_url(&self) -> String;
}

pub trait OutlayerAppDeployerExt {
    /// Deploy a new `outlayer-app` instance via `StateInit`.
    async fn deploy_outlayer_app(
        &self,
        global_contract_id: GlobalContractIdentifier,
        state: OutlayerState<'static>,
    ) -> OutlayerAppClient;
}

impl OutlayerAppDeployerExt for Near {
    async fn deploy_outlayer_app(
        &self,
        global_contract_id: GlobalContractIdentifier,
        state: OutlayerState<'static>,
    ) -> OutlayerAppClient {
        self.contract::<OutlayerApp>(
            self.deploy_deterministic_account(
                global_contract_id,
                state.state_init(),
                NearToken::ZERO,
            )
            .await,
        )
    }
}

pub trait OutlayerAppExt {
    async fn oa_set_code(
        &self,
        target: impl Into<AccountId>,
        old_code_hash: [u8; 32],
        new_code_hash: [u8; 32],
        new_code_url: String,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn oa_transfer_admin(
        &self,
        target: impl Into<AccountId>,
        new_admin_id: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl OutlayerAppExt for Near {
    async fn oa_set_code(
        &self,
        target: impl Into<AccountId>,
        old_code_hash: [u8; 32],
        new_code_hash: [u8; 32],
        new_code_url: String,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(target.into())
            .add_action(
                OutlayerApp::oa_set_code(OaSetCodeArgs {
                    old_code_hash: old_code_hash.into(),
                    new_code_hash: new_code_hash.into(),
                    new_code_url,
                })
                .deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .try_into()
    }

    async fn oa_transfer_admin(
        &self,
        target: impl Into<AccountId>,
        new_admin_id: impl Into<AccountId>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(target.into())
            .add_action(
                OutlayerApp::oa_transfer_admin(OaTransferAdminArgs {
                    new_admin_id: new_admin_id.into(),
                })
                .deposit(NearToken::from_yoctonear(1)),
            )
            .await?
            .try_into()
    }
}
