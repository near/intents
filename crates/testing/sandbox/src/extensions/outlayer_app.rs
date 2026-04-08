use crate::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use defuse_outlayer_app::{State as OutlayerState, Url};
use defuse_serde_utils::hex::AsHex;
use near_api::types::transaction::result::ExecutionSuccess;
use near_sdk::{
    AccountId, GlobalContractId, NearToken,
    serde_json::json,
    state_init::{StateInit, StateInitV1},
};

#[allow(async_fn_in_trait)]
pub trait OutlayerAppExt {
    /// Deploy a new `outlayer-app` instance via `StateInit`.
    async fn deploy_outlayer_app(
        &self,
        global_contract_id: GlobalContractId,
        state: OutlayerState,
    ) -> anyhow::Result<Account>;

    async fn op_approve(
        &self,
        target: &AccountId,
        new_hash: [u8; 32],
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn op_set_admin_id(
        &self,
        target: &AccountId,
        new_admin_id: &AccountId,
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn op_set_code_uri(
        &self,
        target: &AccountId,
        url: Url,
    ) -> anyhow::Result<ExecutionSuccess>;
}

#[allow(async_fn_in_trait)]
pub trait OutlayerAppViewExt {
    async fn op_admin_id(&self) -> anyhow::Result<AccountId>;
    async fn op_code_hash(&self) -> anyhow::Result<[u8; 32]>;
    async fn op_code_uri(&self) -> anyhow::Result<Url>;
}

impl OutlayerAppExt for SigningAccount {
    async fn deploy_outlayer_app(
        &self,
        global_contract_id: GlobalContractId,
        state: OutlayerState,
    ) -> anyhow::Result<Account> {
        let account_id = self
            .state_init(
                StateInit::V1(StateInitV1 {
                    code: global_contract_id,
                    data: state.state_init(),
                }),
                NearToken::ZERO,
            )
            .await?;
        Ok(Account::new(account_id, self.network_config().clone()))
    }

    async fn op_approve(
        &self,
        target: &AccountId,
        new_hash: [u8; 32],
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("op_approve")
                    .json_args(json!({"new_hash": AsHex(new_hash)}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
    }

    async fn op_set_admin_id(
        &self,
        target: &AccountId,
        new_admin_id: &AccountId,
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("op_set_admin_id")
                    .json_args(json!({"new_admin_id": new_admin_id}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
    }

    async fn op_set_code_uri(
        &self,
        target: &AccountId,
        url: Url,
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("op_set_code_uri")
                    .json_args(json!({"url": url}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
    }
}

impl OutlayerAppViewExt for Account {
    async fn op_admin_id(&self) -> anyhow::Result<AccountId> {
        self.call_view_function_json("op_admin_id", ()).await
    }

    async fn op_code_hash(&self) -> anyhow::Result<[u8; 32]> {
        let hash: AsHex<[u8; 32]> = self.call_view_function_json("op_code_hash", ()).await?;
        Ok(hash.into_inner())
    }

    async fn op_code_uri(&self) -> anyhow::Result<Url> {
        self.call_view_function_json("op_code_uri", ()).await
    }
}
