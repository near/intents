use std::{collections::BTreeMap, fs, path::Path, sync::LazyLock};

use defuse_sandbox::{
    api::types::transaction::actions::GlobalContractDeployMode, Account, SigningAccount,
};
use near_sdk::{
    AccountId, NearToken,
    state_init::{StateInit, StateInitV1},
};

#[track_caller]
fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../res/")
        .join(name)
        .with_extension("wasm");
    fs::read(filename.clone()).expect(&format!("file {filename:?} should exists"))
}

pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("multi-token-receiver-stub/multi_token_receiver_stub"));

pub trait MtReceiverStubAccountExt {
    /// Deploy as regular contract
    async fn deploy_mt_receiver_stub(&self, name: impl AsRef<str>) -> Account;
    /// Deploy as global contract (code only)
    async fn deploy_mt_receiver_stub_global(&self, name: impl AsRef<str>) -> AccountId;
    /// Deploy instance referencing global contract
    async fn deploy_mt_receiver_stub_instance(&self, global_contract_id: AccountId) -> AccountId;
}

impl MtReceiverStubAccountExt for SigningAccount {
    async fn deploy_mt_receiver_stub(&self, name: impl AsRef<str>) -> Account {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(20))
            .deploy(MT_RECEIVER_STUB_WASM.clone())
            .no_result()
            .await
            .unwrap();

        account
    }

    async fn deploy_mt_receiver_stub_global(&self, name: impl AsRef<str>) -> AccountId {
        let account = self.subaccount(name);

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(100))
            .deploy_global(
                MT_RECEIVER_STUB_WASM.clone(),
                GlobalContractDeployMode::AccountId,
            )
            .await
            .unwrap();

        account.id().clone()
    }

    async fn deploy_mt_receiver_stub_instance(&self, global_contract_id: AccountId) -> AccountId {
        // The contract is stateless, so we use empty state
        let raw_state: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
        let state_init = StateInit::V1(StateInitV1 {
            code: near_sdk::GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state,
        });

        let account = state_init.derive_account_id();

        // NOTE: there is rpc error on state_init action but the contract itself is successfully
        // deployed, so lets ignore error for now
        let _ = self
            .tx(account.clone())
            .state_init(global_contract_id, BTreeMap::new())
            .transfer(NearToken::from_yoctonear(1))
            .await;

        account
    }
}
