use anyhow::Result;
use borsh::BorshSerialize;
use defuse_global_deployer::{AsHex, State as DeployerState};
use near_account_id::AccountId;
use near_kit::{Final, Gas, GlobalContractIdentifier, Near, NearToken};
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};
use sha2::{Digest, Sha256};

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

// Standard borsh serialization would prepend a 4-byte length
fn serialize_code_remainder<W: std::io::Write>(v: &Vec<u8>, w: &mut W) -> std::io::Result<()> {
    w.write_all(v)
}

#[derive(Serialize, Deserialize)]
pub struct GDTransferOwnershipArgs {
    pub receiver_id: AccountId,
}

// TODO: remove this after near kit fix (passing raw data)
#[derive(BorshSerialize)]
pub struct GDDeployArgs {
    #[borsh(serialize_with = "serialize_code_remainder")]
    pub code: Vec<u8>,
}

#[near_kit::contract]
pub trait GlobalDeployer {
    #[call]
    fn gd_approve(&mut self, args: GDApproveArgs) -> bool;

    #[call]
    #[borsh]
    fn gd_deploy(&mut self, args: GDDeployArgs) -> bool;

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
        global_contract_id: GlobalContractIdentifier,
        state: DeployerState<'_>,
    ) -> Result<GlobalDeployerClient>;
}

impl GDDeployerExt for Near {
    async fn deploy_gd_instance(
        &self,
        global_contract_id: GlobalContractIdentifier,
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
        target: impl Into<AccountId>,
        old_hash: impl Into<[u8; 32]>,
        new_code: impl AsRef<[u8]>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn gd_approve(
        &self,
        target: impl Into<AccountId>,
        old_hash: impl Into<[u8; 32]>,
        new_hash: impl Into<[u8; 32]>,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn gd_deploy(
        &self,
        target: impl Into<AccountId>,
        code: impl AsRef<[u8]>,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn gd_transfer_ownership(
        &self,
        target: impl Into<AccountId>,
        new_owner: impl Into<AccountId>,
    ) -> anyhow::Result<SuccessfulExecutionOutcome>;
}

impl GlobalDeployerExt for Near {
    async fn gd_approve_and_deploy(
        &self,
        target: impl Into<AccountId>,
        old_hash: impl Into<[u8; 32]>,
        new_code: impl AsRef<[u8]>,
    ) -> Result<SuccessfulExecutionOutcome> {
        let code = new_code.as_ref().to_vec();
        let new_hash: [u8; 32] = Sha256::digest(&code).into();

        self.transaction(target.into())
            .add_action(
                GlobalDeployer::gd_approve(GDApproveArgs {
                    old_hash: old_hash.into().into(),
                    new_hash: new_hash.into(),
                })
                .deposit(NearToken::from_yoctonear(1))
                .gas(Gas::from_tgas(10)),
            )
            .add_action(
                GlobalDeployer::gd_deploy(GDDeployArgs { code })
                    .deposit(NearToken::from_near(50))
                    .gas(Gas::from_tgas(290)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn gd_approve(
        &self,
        target: impl Into<AccountId>,
        old_hash: impl Into<[u8; 32]>,
        new_hash: impl Into<[u8; 32]>,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(target.into())
            .add_action(
                GlobalDeployer::gd_approve(GDApproveArgs {
                    old_hash: old_hash.into().into(),
                    new_hash: new_hash.into().into(),
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
        target: impl Into<AccountId>,
        code: impl AsRef<[u8]>,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome> {
        let code = code.as_ref().to_vec();
        self.transaction(target.into())
            .add_action(
                GlobalDeployer::gd_deploy(GDDeployArgs { code })
                    .deposit(deposit)
                    .gas(Gas::from_tgas(290)),
            )
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn gd_transfer_ownership(
        &self,
        target: impl Into<AccountId>,
        new_owner: impl Into<AccountId>,
    ) -> anyhow::Result<SuccessfulExecutionOutcome> {
        self.transaction(target.into())
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
