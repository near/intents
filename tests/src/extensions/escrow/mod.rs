pub use defuse_escrow_swap as contract;

use defuse_sandbox::{Account, SigningAccount, anyhow, api::types::transaction::actions::GlobalContractDeployMode, tx::FnCallBuilder};
use near_sdk::{AccountId, GlobalContractId, NearToken, serde_json::json, state_init::{StateInit, StateInitV1}};

use crate::env::ESCROW_SWAP_WASM;

use defuse_escrow_swap::{Params, Storage};

#[allow(async_fn_in_trait)]
pub trait EscrowExt {
    async fn es_close(
        &self,
        escrow_id: impl Into<AccountId>,
        params: &Params,
    ) -> anyhow::Result<bool>;
    async fn es_lost_found(
        &self,
        escrow_id: impl Into<AccountId>,
        params: &Params,
    ) -> anyhow::Result<bool>;
}

impl EscrowExt for SigningAccount {
    async fn es_close(
        &self,
        escrow_id: impl Into<AccountId>,
        params: &Params,
    ) -> anyhow::Result<bool> {
        self.tx(escrow_id)
            .function_call(FnCallBuilder::new("es_close").json_args(json!({
                "params": params
            })))
            .await?
            .json()
            .map_err(Into::into)
    }

    async fn es_lost_found(
        &self,
        escrow_id: impl Into<AccountId>,
        params: &Params,
    ) -> anyhow::Result<bool> {
        self.tx(escrow_id)
            .function_call(FnCallBuilder::new("es_lost_found").json_args(json!({
                "params": params
            })))
            .await?
            .json()
            .map_err(Into::into)
    }
}

#[allow(async_fn_in_trait)]
pub trait EscrowExtView {
    async fn es_view(&self) -> anyhow::Result<Storage>;
}

impl EscrowExtView for Account {
    async fn es_view(&self) -> anyhow::Result<Storage> {
        self.call_view_function_json("es_view", ()).await
    }
}

pub trait EscrowSwapExt {
    /// Deploy global escrow-swap contract (shared code)
    async fn deploy_escrow_swap_global(&self, name: impl AsRef<str>) -> AccountId;

    /// Deploy an escrow-swap instance with specific params using `state_init`
    async fn deploy_escrow_swap_instance(
        &self,
        global_contract_id: AccountId,
        params: &defuse_escrow_swap::Params,
    ) -> AccountId;
}

impl EscrowSwapExt for SigningAccount {
    async fn deploy_escrow_swap_global(&self, name: impl AsRef<str>) -> AccountId {
        let account = self.sub_account(name).unwrap();

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
        params: &defuse_escrow_swap::Params,
    ) -> AccountId {
        let raw_state = defuse_escrow_swap::ContractStorage::init_state(params).unwrap();
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
