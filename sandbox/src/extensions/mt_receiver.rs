use std::collections::BTreeMap;

use near_sdk::{
    AccountId, GlobalContractId, NearToken, state_init::StateInit, state_init::StateInitV1,
};

use crate::SigningAccount;

pub trait MtReceiverStubExt {
    /// Deploy MT receiver stub as a regular contract (subaccount of self)
    #[allow(clippy::use_self)]
    async fn deploy_mt_receiver_stub(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<SigningAccount>;

    /// Deploy MT receiver stub as a global contract (subaccount of self)
    #[allow(clippy::use_self)]
    async fn deploy_mt_receiver_stub_global(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<SigningAccount>;

    /// Deploy an instance using `DeterministicStateInit` with the given raw state.
    /// Returns the deterministic account ID derived from the state.
    async fn deploy_mt_receiver_stub_instance(
        &self,
        global_contract_id: AccountId,
        raw_state: BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> anyhow::Result<AccountId>;
}

impl MtReceiverStubExt for SigningAccount {
    async fn deploy_mt_receiver_stub(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<Self> {
        self.deploy_sub_contract(name, NearToken::from_near(10), wasm.into(), None)
            .await
    }

    async fn deploy_mt_receiver_stub_global(
        &self,
        name: impl AsRef<str>,
        wasm: impl Into<Vec<u8>>,
    ) -> anyhow::Result<Self> {
        self.deploy_global_sub_contract(name, NearToken::from_near(100), wasm.into())
            .await
    }

    async fn deploy_mt_receiver_stub_instance(
        &self,
        global_contract_id: AccountId,
        raw_state: BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> anyhow::Result<AccountId> {
        let state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state.clone(),
        });

        let account = state_init.derive_account_id();

        self.tx(account.clone())
            .state_init(global_contract_id, raw_state)
            .transfer(NearToken::from_yoctonear(1))
            .await?;

        Ok(account)
    }
}
