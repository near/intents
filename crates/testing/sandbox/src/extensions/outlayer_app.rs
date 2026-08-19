use anyhow::Result;
use defuse_outlayer_app::{
    State as OutlayerState,
    client::{OaSetCodeArgs, OutlayerAppContract, OutlayerAppContractClient},
};
use near_kit::{AccountIdRef, Final, Gas, GlobalContractId, Near, NearToken};

use crate::{nep616::DeployDeterministicAccountExt, outcome::SuccessfulExecutionOutcome};

pub use defuse_outlayer_app as contract;

pub trait OutlayerAppDeployerExt {
    /// Deploy a new `outlayer-app` instance via `StateInit`.
    async fn deploy_outlayer_app(
        &self,
        global_contract_id: GlobalContractId,
        state: OutlayerState<'static>,
    ) -> OutlayerAppContractClient;
}

impl OutlayerAppDeployerExt for Near {
    async fn deploy_outlayer_app(
        &self,
        global_contract_id: GlobalContractId,
        state: OutlayerState<'static>,
    ) -> OutlayerAppContractClient {
        self.contract::<OutlayerAppContract>(
            self.deploy_deterministic_account(
                global_contract_id,
                state.as_storage(),
                NearToken::ZERO,
            )
            .await
            .unwrap(),
        )
    }
}

pub trait OutlayerAppExt {
    async fn oa_set_code(
        &self,
        target: impl AsRef<AccountIdRef>,
        old_code_hash: [u8; 32],
        new_code_hash: [u8; 32],
        new_code_url: impl AsRef<str>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn oa_transfer_admin(
        &self,
        target: impl AsRef<AccountIdRef>,
        new_admin_id: impl AsRef<AccountIdRef>,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl OutlayerAppExt for Near {
    async fn oa_set_code(
        &self,
        target: impl AsRef<AccountIdRef>,
        old_code_hash: [u8; 32],
        new_code_hash: [u8; 32],
        new_code_url: impl AsRef<str>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(target.as_ref())
            .add_action(
                OutlayerAppContract::oa_set_code(OaSetCodeArgs {
                    old_code_hash,
                    new_code_hash,
                    new_code_url: new_code_url.as_ref().into(),
                })
                .deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(10)),
            )
            .wait_until::<Final>()
            .await?
            .try_into()
    }

    async fn oa_transfer_admin(
        &self,
        target: impl AsRef<AccountIdRef>,
        new_admin_id: impl AsRef<AccountIdRef>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(target.as_ref())
            .add_action(
                OutlayerAppContract::oa_transfer_admin(new_admin_id.as_ref().into())
                    .deposit(NearToken::from_yoctonear(1))
                    .gas(Gas::from_tgas(30)),
            )
            .wait_until::<Final>()
            .await?
            .try_into()
    }
}
