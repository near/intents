use anyhow::Result;
use near_kit::{
    DeterministicAccountStateInit, DeterministicAccountStateInitV1, Final,
    GlobalContractIdentifier, Near,
};
use near_sdk::{AccountId, NearToken};
use std::collections::BTreeMap;

pub trait DeployDeterministicAccountExt {
    /// Deploy a new global contract instance via `StateInit`.
    async fn deploy_deterministic_account(
        &self,
        global_contract_id: GlobalContractIdentifier,
        state: impl Into<BTreeMap<Vec<u8>, Vec<u8>>>,
        deposit: NearToken,
    ) -> Result<AccountId>;
}

impl DeployDeterministicAccountExt for Near {
    async fn deploy_deterministic_account(
        &self,
        global_contract_id: GlobalContractIdentifier,
        state: impl Into<BTreeMap<Vec<u8>, Vec<u8>>>,
        deposit: NearToken,
    ) -> Result<AccountId> {
        let si = DeterministicAccountStateInit::V1(DeterministicAccountStateInitV1 {
            code: global_contract_id,
            data: state.into(),
        });
        let account_id = si.derive_account_id();

        self.state_init(si, deposit)
            .send()
            .wait_until(Final)
            .await?
            .result()?;

        Ok(account_id)
    }
}
