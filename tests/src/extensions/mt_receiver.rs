use std::collections::BTreeMap;

use near_sdk::{
    AccountId, GlobalContractId, NearToken,
    state_init::{StateInit, StateInitV1},
};

use defuse_sandbox::{SigningAccount, api::types::transaction::actions::GlobalContractDeployMode};

use crate::env::MT_RECEIVER_STUB_WASM;

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
