use std::{fs, path::Path, sync::LazyLock};

use defuse_sandbox::{SigningAccount, api::types::transaction::actions::GlobalContractDeployMode};
use near_sdk::{
    AccountId, GlobalContractId, NearToken,
    state_init::{StateInit, StateInitV1},
};

pub static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR")).join("../res/defuse_escrow_swap.wasm");
    fs::read(filename.clone()).unwrap_or_else(|_| panic!("file {filename:?} should exists"))
});

#[allow(async_fn_in_trait)]
pub trait EscrowSwapAccountExt {
    /// Deploy global escrow-swap contract (shared code)
    async fn deploy_escrow_swap_global(&self, name: impl AsRef<str>) -> AccountId;

    /// Deploy an escrow-swap instance with specific params using `state_init`
    async fn deploy_escrow_swap_instance(
        &self,
        global_contract_id: AccountId,
        params: &defuse_escrow_swap::Params,
    ) -> AccountId;
}

impl EscrowSwapAccountExt for SigningAccount {
    async fn deploy_escrow_swap_global(&self, name: impl AsRef<str>) -> AccountId {
        let account = self.sub_account(name).unwrap();

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(50))
            .deploy_global(ESCROW_SWAP_WASM.clone(), GlobalContractDeployMode::AccountId)
            .await
            .unwrap();

        account.id().clone()
    }

    async fn deploy_escrow_swap_instance(
        &self,
        global_contract_id: AccountId,
        params: &defuse_escrow_swap::Params,
    ) -> AccountId {
        let raw_state = defuse_escrow_swap::ContractStorage::init_state(params).unwrap();
        let state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state.clone(),
        });
        let account_id = state_init.derive_account_id();

        // Note: RPC may error but contract deploys successfully
        let _ = self
            .tx(account_id.clone())
            .state_init(global_contract_id, raw_state)
            .transfer(NearToken::from_yoctonear(1))
            .await;
        account_id
    }
}
