use std::collections::BTreeMap;

use near_api::types::transaction::result::ExecutionFinalResult;
use near_sdk::{
    AccountId, GlobalContractId, NearToken, state_init::StateInit, state_init::StateInitV1,
};

use crate::{Account, SigningAccount};

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

    /// Deploy an instance and return the full execution result for gas analysis.
    /// Returns the deterministic account ID and the execution result.
    async fn deploy_mt_receiver_stub_instance_raw(
        &self,
        global_contract_id: AccountId,
        raw_state: BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> anyhow::Result<(AccountId, ExecutionFinalResult)>;
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
        let (account, _) = self
            .deploy_mt_receiver_stub_instance_raw(global_contract_id, raw_state)
            .await?;
        Ok(account)
    }

    async fn deploy_mt_receiver_stub_instance_raw(
        &self,
        global_contract_id: AccountId,
        raw_state: BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> anyhow::Result<(AccountId, ExecutionFinalResult)> {
        let state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(global_contract_id.clone()),
            data: raw_state.clone(),
        });

        let account = state_init.derive_account_id();

        let result = self
            .tx(account.clone())
            .state_init(state_init, NearToken::ZERO)
            .exec_transaction()
            .await?;

        Ok((account, result))
    }
}

#[allow(async_fn_in_trait)]
pub trait MtReceiverStubExtView {
    async fn dummy_method(&self) -> anyhow::Result<()>;
}

impl MtReceiverStubExtView for Account {
    async fn dummy_method(&self) -> anyhow::Result<()> {
        self.call_view_function_raw("dummy_method", ()).await?;
        Ok(())
    }
}
