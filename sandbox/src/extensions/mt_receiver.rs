use std::collections::BTreeMap;
use std::{fs, path::Path, sync::LazyLock};

use near_sdk::{
    AccountId, GlobalContractId, NearToken,
    state_init::{StateInit, StateInitV1},
};

use crate::{SigningAccount, api::types::transaction::actions::GlobalContractDeployMode};

pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../res/multi-token-receiver-stub/multi_token_receiver_stub.wasm");
    fs::read(filename.clone()).unwrap_or_else(|_| panic!("file {filename:?} should exist"))
});

pub trait MtReceiverStubExt {
    /// Deploy as global contract (code only)
    async fn deploy_mt_receiver_stub_global(&self, name: impl AsRef<str>) -> AccountId;
    /// Deploy instance referencing global contract with arbitrary raw state
    async fn deploy_mt_receiver_stub_instance(
        &self,
        global_contract_id: AccountId,
        raw_state: BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> AccountId;
}

impl MtReceiverStubExt for SigningAccount {
    async fn deploy_mt_receiver_stub_global(&self, name: impl AsRef<str>) -> AccountId {
        let account = self.sub_account(name).unwrap();

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

    async fn deploy_mt_receiver_stub_instance(
        &self,
        global_contract_id: AccountId,
        raw_state: BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> AccountId {
        let state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state.clone(),
        });

        let account = state_init.derive_account_id();

        self.tx(account.clone())
            .state_init(state_init)
            .transfer(NearToken::from_yoctonear(1))
            .await
            .unwrap();

        account
    }
}
