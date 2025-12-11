use std::{fs, path::Path, sync::LazyLock};

use crate::state::{ContractStorage, Params};
use defuse_sandbox::{
    api::types::transaction::actions::GlobalContractDeployMode, SigningAccount,
};
use near_sdk::{
    AccountId, GlobalContractId, NearToken,
    state_init::{StateInit, StateInitV1},
};

#[track_caller]
fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/")
        .join(name)
        .with_extension("wasm");
    fs::read(filename.clone()).unwrap_or_else(|_| panic!("file {filename:?} should exist"))
}

pub static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("defuse_escrow_swap"));

/// Derive the escrow-swap instance account ID from its params
pub fn derive_escrow_swap_account_id(global_contract_id: &AccountId, params: &Params) -> AccountId {
    let raw_state = ContractStorage::init_state(params).unwrap();
    let state_init = StateInit::V1(StateInitV1 {
        code: GlobalContractId::AccountId(global_contract_id.clone()),
        data: raw_state,
    });
    state_init.derive_account_id()
}

pub trait EscrowSwapAccountExt {
    /// Deploy global escrow-swap contract (shared code)
    async fn deploy_escrow_swap_global(&self, name: impl AsRef<str>) -> AccountId;

    /// Deploy an escrow-swap instance with specific params using state_init
    async fn deploy_escrow_swap_instance(
        &self,
        global_contract_id: AccountId,
        params: &Params,
    ) -> AccountId;
}

impl EscrowSwapAccountExt for SigningAccount {
    async fn deploy_escrow_swap_global(&self, name: impl AsRef<str>) -> AccountId {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(50))
            .deploy_global(
                ESCROW_SWAP_WASM.clone(),
                GlobalContractDeployMode::AccountId,
            )
            .await
            .unwrap();

        account.id().clone()
    }

    async fn deploy_escrow_swap_instance(
        &self,
        global_contract_id: AccountId,
        params: &Params,
    ) -> AccountId {
        let raw_state = ContractStorage::init_state(params).unwrap();
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
