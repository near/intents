use anyhow::Result;
use defuse_digest::{Digest, sha2::Sha256};
use defuse_global_deployer::{AsHex, AsWrap, Remainder, State as DeployerState};
use near_kit::{AccountId, AccountIdRef, Final, Gas, GlobalContractId, Near, NearToken};
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};

use crate::{nep616::DeployDeterministicAccountExt, outcome::SuccessfulExecutionOutcome};

pub use defuse_global_deployer as contract;

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct GDApproveArgs {
    #[serde_as(as = "Hex")]
    pub old_hash: [u8; 32],
    #[serde_as(as = "Hex")]
    pub new_hash: [u8; 32],
}

#[derive(Serialize, Deserialize)]
pub struct GDTransferOwnershipArgs {
    pub receiver_id: AccountId,
}

#[near_kit::contract]
pub trait GlobalDeployer {
    #[call]
    fn gd_approve(&mut self, args: GDApproveArgs) -> bool;

    #[call]
    #[borsh]
    fn gd_deploy(&mut self, code: AsWrap<Vec<u8>, Remainder>) -> bool;

    #[call]
    fn gd_transfer_ownership(&mut self, args: GDTransferOwnershipArgs);

    fn gd_owner_id(&self) -> AccountId;
    fn gd_code_hash(&self) -> AsHex<[u8; 32]>;
    fn gd_approved_hash(&self) -> AsHex<[u8; 32]>;
}

pub trait GDDeployerExt {
    /// Deploy a new `global-deployer` instance via `StateInit`.
    async fn deploy_gd_instance(
        &self,
        global_contract_id: GlobalContractId,
        state: DeployerState<'_>,
    ) -> Result<GlobalDeployerClient>;
}

impl GDDeployerExt for Near {
    async fn deploy_gd_instance(
        &self,
        global_contract_id: GlobalContractId,
        state: DeployerState<'_>,
    ) -> Result<GlobalDeployerClient> {
        Ok(self.contract::<GlobalDeployer>(
            self.deploy_deterministic_account(
                global_contract_id,
                state.state_init(),
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
        new_owner: impl Into<AccountId>,
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
                GlobalDeployer::gd_approve(GDApproveArgs {
                    old_hash: old_hash.into(),
                    new_hash: Sha256::digest(&code).into(),
                })
                .deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(10)),
            )
            .add_action(
                GlobalDeployer::gd_deploy(AsWrap::new(code))
                    .deposit(NearToken::from_near(50))
                    .gas(Gas::from_tgas(290)),
            )
            .wait_until(Final)
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
                GlobalDeployer::gd_approve(GDApproveArgs {
                    old_hash: old_hash.into(),
                    new_hash: new_hash.into(),
                })
                .deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(10)),
            )
            .wait_until(Final)
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
                GlobalDeployer::gd_deploy(AsWrap::new(code))
                    .deposit(deposit)
                    .gas(Gas::from_tgas(290)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn gd_transfer_ownership(
        &self,
        target: impl AsRef<AccountIdRef>,
        new_owner: impl Into<AccountId>,
    ) -> anyhow::Result<SuccessfulExecutionOutcome> {
        self.transaction(target.as_ref())
            .add_action(
                GlobalDeployer::gd_transfer_ownership(GDTransferOwnershipArgs {
                    receiver_id: new_owner.into(),
                })
                .deposit(NearToken::from_yoctonear(1)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }
}
