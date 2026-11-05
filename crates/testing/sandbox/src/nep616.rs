use anyhow::Result;
use near_kit::{
    AccountId, NearToken, SigningNear,
    protocol::{Action, GlobalContractId, StateInit, StateInitV1},
};
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

impl DeployDeterministicAccountExt for SigningNear {
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

        self.transaction(&account_id)
            .add_action(Action::state_init(si, deposit))
            .await?
            .result()?;

        Ok(account_id)
    }
}
