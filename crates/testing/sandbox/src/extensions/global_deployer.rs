use anyhow::Result;
use defuse_digest::{Digest, sha2::Sha256};
use defuse_global_deployer::{
    AsHex, AsWrap, State as DeployerState,
    client::{GlobalDeployerContract, GlobalDeployerContractClient},
};
use near_kit::{
    AccountIdRef, Gas, NearToken, SigningNear, protocol::GlobalContractId, transaction::Final,
};

use crate::{nep616::DeployDeterministicAccountExt, outcome::SuccessfulExecutionOutcome};

pub use defuse_global_deployer as contract;

pub trait GDDeployerExt: Sized {
    /// Deploy a new `global-deployer` instance via `StateInit`.
    async fn deploy_gd_instance(
        &self,
        global_contract_id: GlobalContractId,
        state: DeployerState<'_>,
    ) -> Result<GlobalDeployerContractClient<Self>>;
}

impl GDDeployerExt for SigningNear {
    async fn deploy_gd_instance(
        &self,
        global_contract_id: GlobalContractId,
        state: DeployerState<'_>,
    ) -> Result<GlobalDeployerContractClient<Self>> {
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

impl GlobalDeployerExt for SigningNear {
    async fn gd_approve_and_deploy(
        &self,
        target: impl AsRef<AccountIdRef>,
        old_hash: impl Into<[u8; 32]>,
        new_code: &[u8],
    ) -> Result<SuccessfulExecutionOutcome> {
        let code = new_code.to_vec();

        self.transaction(target.as_ref())
            .add_action(
                GlobalDeployerContract::gd_approve(
                    AsHex(old_hash.into()),
                    AsHex(Sha256::digest(&code).into()),
                )
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
                GlobalDeployerContract::gd_approve(AsHex(old_hash.into()), AsHex(new_hash.into()))
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
                GlobalDeployerContract::gd_transfer_ownership(new_owner.as_ref())
                    .gas(Gas::from_tgas(30)),
            )
            .wait_until::<Final>()
            .await?
            .try_into()
    }
}
