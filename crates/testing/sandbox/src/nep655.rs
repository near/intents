use anyhow::Result;
use near_kit::{
    AccountId, NearToken, SigningNear,
    protocol::{Action, GlobalContractId, UniversalStateInit, UniversalStateInitV1},
};

use std::collections::{BTreeMap, BTreeSet};

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
        let si = UniversalStateInit::V1(UniversalStateInitV1 {
            code: Some(global_contract_id),
            data: state.into(),
            access_keys: BTreeSet::new(),
        })
        .to_raw();
        let account_id = si.try_derive_account_id();

        self.transaction(&account_id)
            .add_action(Action::universal_state_init(si, deposit))
            .await?
            .checked()?;

        Ok(account_id)
    }
}
