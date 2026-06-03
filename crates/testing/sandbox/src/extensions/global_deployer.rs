use defuse_global_deployer::{AsHex, State as DeployerState};
use near_kit::{
    DeterministicAccountStateInit, DeterministicAccountStateInitV1, Finality,
    GlobalContractIdentifier,
};
use near_sdk::{
    AccountId, Gas, NearToken,
    borsh::BorshSerialize,
    serde::{Deserialize, Serialize},
};

use crate::{IntoAccountId, Sandbox};

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct GDApproveArgs {
    pub old_hash: AsHex<[u8; 32]>,
    pub new_hash: AsHex<[u8; 32]>,
}

// Standard borsh serialization would prepend a 4-byte length
fn serialize_code_remainder<W: std::io::Write>(v: &Vec<u8>, w: &mut W) -> std::io::Result<()> {
    w.write_all(v)
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct GDTransferOwnershipArgs {
    pub receiver_id: AccountId,
}

// TODO: remove this after near kit fix (passing raw data)
#[derive(BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
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

impl Sandbox {
    pub async fn deploy_gd_instance(
        &self,
        global_contract_id: GlobalContractIdentifier,
        state: DeployerState,
    ) -> anyhow::Result<GlobalDeployerClient> {
        let state_init = DeterministicAccountStateInit::V1(DeterministicAccountStateInitV1 {
            code: global_contract_id,
            data: state.state_init(),
        });
        let deterministic_account_id = state_init.derive_account_id();

        self.transaction(self.into_account_id())
            .state_init(state_init, NearToken::ZERO)
            .await?;

        Ok(self.contract::<dyn GlobalDeployer>(deterministic_account_id))
    }

    pub async fn gd_approve_and_deploy(
        &self,
        gd: &GlobalDeployerClient,
        old_hash: [u8; 32],
        new_code: &[u8],
    ) -> anyhow::Result<()> {
        let new_hash = near_sdk::env::sha256_array(new_code);

        // TODO: merge it into single transaction after fix in near kit
        gd.gd_approve(GDApproveArgs {
            old_hash: AsHex(old_hash),
            new_hash: AsHex(new_hash),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(10))
        .await?;

        gd.gd_deploy(GDDeployArgs {
            code: new_code.to_vec(),
        })
        .deposit(NearToken::from_near(50))
        .gas(Gas::from_tgas(290))
        .await?;

        Ok(())
    }

    pub async fn global_contract_id(
        &self,
        gd_id: impl Into<AccountId>,
    ) -> anyhow::Result<GlobalContractIdentifier> {
        let account = self
            .account(gd_id.into())
            .finality(Finality::Optimistic)
            .await?;

        // TODO: i dont like it
        if let Some(global_contract_account_id) = account.global_contract_account_id {
            return Ok(GlobalContractIdentifier::AccountId(
                global_contract_account_id,
            ));
        }

        if let Some(global_contract_hash) = account.global_contract_hash {
            return Ok(GlobalContractIdentifier::CodeHash(global_contract_hash));
        }

        anyhow::bail!("Account is not a global contract")
    }
}
