use std::{fs, path::Path, sync::LazyLock};

use defuse_escrow_proxy::ProxyConfig;
#[cfg(feature = "escrow-swap")]
use defuse_escrow_swap::Params as EscrowParams;
use near_sdk::{
    AccountId, Gas, GlobalContractId, NearToken,
    state_init::{StateInit, StateInitV1},
};
use serde_json::json;

use crate::{
    FnCallBuilder, SigningAccount, api::types::transaction::actions::GlobalContractDeployMode,
};

pub static ESCROW_PROXY_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR")).join("../res/defuse_escrow_proxy.wasm");
    fs::read(filename.clone()).unwrap_or_else(|_| panic!("file {filename:?} should exist"))
});

pub trait EscrowProxyExt {
    async fn deploy_escrow_proxy(&self, config: ProxyConfig) -> anyhow::Result<()>;
    async fn get_escrow_proxy_config(&self) -> anyhow::Result<ProxyConfig>;
    /// Call `es_cancel` on proxy contract. Requires caller to be owner.
    #[cfg(feature = "escrow-swap")]
    async fn es_cancel(
        &self,
        proxy_contract: &AccountId,
        escrow_address: &AccountId,
        params: &EscrowParams,
    ) -> anyhow::Result<()>;
    /// Deploy global escrow-proxy contract (shared code)
    async fn deploy_escrow_proxy_global(&self, name: impl AsRef<str>) -> AccountId;
    /// Deploy an escrow-proxy instance with specific config using `state_init`
    async fn deploy_escrow_proxy_instance(
        &self,
        global_contract_id: AccountId,
        config: ProxyConfig,
    ) -> AccountId;
}

impl EscrowProxyExt for SigningAccount {
    async fn deploy_escrow_proxy(&self, config: ProxyConfig) -> anyhow::Result<()> {
        self.tx(self.id().clone())
            .transfer(NearToken::from_near(5))
            .deploy(ESCROW_PROXY_WASM.clone())
            .function_call(
                FnCallBuilder::new("new")
                    .json_args(json!({
                        "config": config,
                    }))
                    .with_gas(Gas::from_tgas(50)),
            )
            .await?;

        Ok(())
    }

    async fn get_escrow_proxy_config(&self) -> anyhow::Result<ProxyConfig> {
        self.call_view_function_json("config", json!({})).await
    }

    #[cfg(feature = "escrow-swap")]
    async fn es_cancel(
        &self,
        proxy_contract: &AccountId,
        escrow_address: &AccountId,
        params: &EscrowParams,
    ) -> anyhow::Result<()> {
        self.tx(proxy_contract.clone())
            .function_call(
                FnCallBuilder::new("es_cancel")
                    .json_args(json!({
                        "contract_id": escrow_address,
                        "params": params,
                    }))
                    .with_gas(Gas::from_tgas(100))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;
        Ok(())
    }

    async fn deploy_escrow_proxy_global(&self, name: impl AsRef<str>) -> AccountId {
        let account = self.sub_account(name).unwrap();

        self.tx(account.id().clone())
            .create_account()
            .transfer(NearToken::from_near(50))
            .deploy_global(
                ESCROW_PROXY_WASM.clone(),
                GlobalContractDeployMode::AccountId,
            )
            .await
            .unwrap();

        account.id().clone()
    }

    async fn deploy_escrow_proxy_instance(
        &self,
        global_contract_id: AccountId,
        config: ProxyConfig,
    ) -> AccountId {
        let raw_state = defuse_escrow_proxy::ContractStorage::init_state(config);
        let state_init = StateInit::V1(StateInitV1 {
            code: GlobalContractId::AccountId(global_contract_id),
            data: raw_state,
        });
        let account_id = state_init.derive_account_id();

        // Note: RPC may error but contract deploys successfully
        let _ = self
            .tx(account_id.clone())
            .state_init(state_init)
            .transfer(NearToken::from_yoctonear(1))
            .await;
        account_id
    }
}
