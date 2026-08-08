use anyhow::Result;
use defuse_digest::{Digest, sha2::Sha256};
use defuse_global_deployer::{
    AsWrap, State as DeployerState,
    client::{GdApproveArgs, GlobalDeployerContract, GlobalDeployerContractClient},
};
use near_kit::{AccountIdRef, Final, Gas, GlobalContractId, Near, NearToken};

use crate::{nep616::DeployDeterministicAccountExt, outcome::SuccessfulExecutionOutcome};

pub use defuse_global_deployer as contract;

pub trait GDDeployerExt {
    /// Deploy a new `global-deployer` instance via `StateInit`.
    async fn deploy_gd_instance(
        &self,
        global_contract_id: GlobalContractId,
        state: DeployerState<'_>,
    ) -> Result<GlobalDeployerContractClient>;
}

impl GDDeployerExt for Near {
    async fn deploy_gd_instance(
        &self,
        global_contract_id: GlobalContractId,
        state: DeployerState<'_>,
    ) -> Result<GlobalDeployerContractClient> {
        Ok(self.contract::<GlobalDeployerContract>(
            self.deploy_deterministic_account(
                global_contract_id,
                state.as_storage(),
                NearToken::ZERO,
            )
            .await?,
        ))
    }
}

pub trait GlobalDeployerExt {
    async fn gd_approve_and_deploy(
        &self,
        target: impl AsRef<AccountIdRef>,
        old_hash: impl Into<[u8; 32]>,
        new_code: &[u8],
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn gd_approve(
        &self,
        target: impl AsRef<AccountIdRef>,
        old_hash: impl Into<[u8; 32]>,
        new_hash: impl Into<[u8; 32]>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn gd_deploy(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: &[u8],
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn gd_transfer_ownership(
        &self,
        target: impl AsRef<AccountIdRef>,
        new_owner: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<SuccessfulExecutionOutcome>;
}

impl GlobalDeployerExt for Near {
    async fn gd_approve_and_deploy(
        &self,
        target: impl AsRef<AccountIdRef>,
        old_hash: impl Into<[u8; 32]>,
        new_code: &[u8],
    ) -> Result<SuccessfulExecutionOutcome> {
        let code = new_code.to_vec();

        self.transaction(target.as_ref())
            .add_action(
                GlobalDeployerContract::gd_approve(GdApproveArgs {
                    old_hash: old_hash.into(),
                    new_hash: Sha256::digest(&code).into(),
                })
                .deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(10)),
            )
            .add_action(
                GlobalDeployerContract::gd_deploy(AsWrap::new(code))
                    .deposit(NearToken::from_near(50))
                    .gas(Gas::from_tgas(290)),
            )
            .wait_until::<Final>()
            .await?
            .try_into()
    }

    async fn gd_approve(
        &self,
        target: impl AsRef<AccountIdRef>,
        old_hash: impl Into<[u8; 32]>,
        new_hash: impl Into<[u8; 32]>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(target.as_ref())
            .add_action(
                GlobalDeployerContract::gd_approve(GdApproveArgs {
                    old_hash: old_hash.into(),
                    new_hash: new_hash.into(),
                })
                .deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(10)),
            )
            .wait_until::<Final>()
            .await?
            .try_into()
    }

    async fn gd_deploy(
        &self,
        target: impl AsRef<AccountIdRef>,
        code: &[u8],
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome> {
        let code = code.to_vec();
        self.transaction(target.as_ref())
            .add_action(
                GlobalDeployerContract::gd_deploy(AsWrap::new(code))
                    .deposit(deposit)
                    .gas(Gas::from_tgas(290)),
            )
            .wait_until::<Final>()
            .await?
            .try_into()
    }

    async fn gd_transfer_ownership(
        &self,
        target: impl AsRef<AccountIdRef>,
        new_owner: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<SuccessfulExecutionOutcome> {
        self.transaction(target.as_ref())
            .add_action(
                GlobalDeployerContract::gd_transfer_ownership(new_owner.as_ref().into())
                    .deposit(NearToken::from_yoctonear(1))
                    .gas(Gas::from_tgas(30)),
            )
            .wait_until::<Final>()
            .await?
            .try_into()
    }
}
