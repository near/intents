use anyhow::Result;
use near_account_id::AccountId;
use near_kit::{Final, GlobalContractId, Near, NearToken, StateInit, StateInitV1};
use std::collections::BTreeMap;

pub trait DeployDeterministicAccountExt {
    /// Deploy a new global contract instance via `StateInit`.
    async fn deploy_deterministic_account(
        &self,
        global_contract_id: GlobalContractId,
        state: impl Into<BTreeMap<Vec<u8>, Vec<u8>>>,
        deposit: NearToken,
    ) -> Result<AccountId>;
}

impl DeployDeterministicAccountExt for Near {
    async fn deploy_deterministic_account(
        &self,
        global_contract_id: GlobalContractId,
        state: impl Into<BTreeMap<Vec<u8>, Vec<u8>>>,
        deposit: NearToken,
    ) -> Result<AccountId> {
        let si = StateInit::V1(StateInitV1 {
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
