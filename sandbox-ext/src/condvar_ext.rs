use std::{fs, path::Path, sync::LazyLock};

use defuse_oneshot_condvar::storage::{ContractStorage, State as OneshotCondVarState};
use defuse_sandbox::{
    Account, SigningAccount, api::types::transaction::actions::GlobalContractDeployMode,
};
use near_sdk::{
    AccountId, GlobalContractId, NearToken,
    state_init::{StateInit, StateInitV1},
};
use serde_json::json;

// Re-export StateInit type for convenience (used to deploy oneshot-condvar instances)
pub use defuse_oneshot_condvar::storage::StateInit as State;

pub static ONESHOT_CONDVAR_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR")).join("../res/defuse_oneshot_condvar.wasm");
    fs::read(filename.clone()).unwrap_or_else(|_| panic!("file {filename:?} should exist"))
});

#[allow(async_fn_in_trait)]
pub trait OneshotCondVarAccountExt {
    async fn deploy_oneshot_condvar(&self, name: impl AsRef<str>) -> AccountId;
    async fn deploy_oneshot_condvar_instance(
        &self,
        global_contract_id: AccountId,
        state: State,
    ) -> AccountId;
    async fn get_oneshot_condvar_instance_state(
        &self,
        global_contract_id: AccountId,
    ) -> anyhow::Result<OneshotCondVarState>;
}

impl OneshotCondVarAccountExt for SigningAccount {
    async fn deploy_oneshot_condvar(&self, name: impl AsRef<str>) -> AccountId {
        let account = self.sub_account(name).unwrap();

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(20))
            .deploy_global(
                ONESHOT_CONDVAR_WASM.clone(),
                GlobalContractDeployMode::AccountId,
            )
            .await
            .unwrap();

        account.id().clone()
    }

    async fn deploy_oneshot_condvar_instance(
        &self,
        global_contract_id: AccountId,
        state: State,
    ) -> AccountId {
        let raw_state = ContractStorage::init_state(state.clone()).unwrap();
        let solver1_state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state.clone(),
        });

        let account = solver1_state_init.derive_account_id();

        self.tx(account.clone())
            .state_init(global_contract_id, raw_state)
            .transfer(NearToken::from_yoctonear(1))
            .await
            .unwrap();
        account
    }

    async fn get_oneshot_condvar_instance_state(
        &self,
        global_contract_id: AccountId,
    ) -> anyhow::Result<OneshotCondVarState> {
        let account = Account::new(global_contract_id, self.network_config().clone());
        account.call_view_function_json("view", json!({})).await
    }
}
