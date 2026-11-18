use anyhow::Result;
use defuse_outlayer_app::{
    State as OutlayerState,
    client::{AsHex, OutlayerAppContract, OutlayerAppContractClient},
};
use near_kit::{
    AccountIdRef, Gas, NearToken, SigningNear, protocol::GlobalContractId, rpc::CheckedOutcome,
};

use crate::{extensions::FnCallTransaction, nep616::DeployDeterministicAccountExt};

pub use defuse_outlayer_app as contract;

pub trait OutlayerAppDeployerExt: Sized {
    /// Deploy a new `outlayer-app` instance via `StateInit`.
    async fn deploy_outlayer_app(
        &self,
        global_contract_id: GlobalContractId,
        state: OutlayerState<'static>,
    ) -> OutlayerAppContractClient<Self>;
}

impl OutlayerAppDeployerExt for SigningNear {
    async fn deploy_outlayer_app(
        &self,
        global_contract_id: GlobalContractId,
        state: OutlayerState<'static>,
    ) -> OutlayerAppContractClient<Self> {
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
    ) -> Result<CheckedOutcome>;

    async fn oa_transfer_admin(
        &self,
        target: impl AsRef<AccountIdRef>,
        new_admin_id: impl AsRef<AccountIdRef>,
    ) -> Result<CheckedOutcome>;
}

impl OutlayerAppExt for SigningNear {
    async fn oa_set_code(
        &self,
        target: impl AsRef<AccountIdRef>,
        old_code_hash: [u8; 32],
        new_code_hash: [u8; 32],
        new_code_url: impl AsRef<str>,
    ) -> Result<CheckedOutcome> {
        self.fn_call(
            target.as_ref(),
            OutlayerAppContract::oa_set_code(
                AsHex(old_code_hash),
                AsHex(new_code_hash),
                new_code_url.as_ref(),
            )
            .gas(Gas::from_tgas(10)),
        )
        .await
    }

    async fn oa_transfer_admin(
        &self,
        target: impl AsRef<AccountIdRef>,
        new_admin_id: impl AsRef<AccountIdRef>,
    ) -> Result<CheckedOutcome> {
        self.fn_call(
            target.as_ref(),
            OutlayerAppContract::oa_transfer_admin(new_admin_id.as_ref()).gas(Gas::from_tgas(30)),
        )
        .await
    }
}
