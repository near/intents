use crate::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use defuse_outlayer_project::{State as OutlayerState, WasmLocation};
use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, GlobalContractId, NearToken,
    serde_json::json,
    state_init::{StateInit, StateInitV1},
};

#[allow(async_fn_in_trait)]
pub trait OutlayerProjectExt {
    /// Deploy a new `outlayer-project` instance via `StateInit`.
    async fn deploy_outlayer_project(
        &self,
        global_contract_id: GlobalContractId,
        state: OutlayerState,
    ) -> anyhow::Result<Account>;

    async fn oc_approve(&self, target: &AccountId, new_hash: [u8; 32]) -> anyhow::Result<()>;

    async fn oc_upload_wasm(&self, target: &AccountId, code: &[u8]) -> anyhow::Result<()>;

    async fn oc_set_updater_id(
        &self,
        target: &AccountId,
        new_updater_id: &AccountId,
    ) -> anyhow::Result<()>;

    async fn oc_set_location(
        &self,
        target: &AccountId,
        location: WasmLocation,
    ) -> anyhow::Result<()>;
}

#[allow(async_fn_in_trait)]
pub trait OutlayerProjectViewExt {
    async fn oc_updater_id(&self) -> anyhow::Result<AccountId>;
    async fn oc_wasm_hash(&self) -> anyhow::Result<[u8; 32]>;
    async fn oc_wasm(&self) -> anyhow::Result<Option<Vec<u8>>>;
    async fn oc_location(&self) -> anyhow::Result<Option<WasmLocation>>;
}

impl OutlayerProjectExt for SigningAccount {
    async fn deploy_outlayer_project(
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

    async fn oc_approve(&self, target: &AccountId, new_hash: [u8; 32]) -> anyhow::Result<()> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("oc_approve")
                    .json_args(json!({"new_hash": AsHex(new_hash)}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;
        Ok(())
    }

    async fn oc_upload_wasm(&self, target: &AccountId, code: &[u8]) -> anyhow::Result<()> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("oc_upload_wasm")
                    .raw_args(code.to_vec())
                    .with_deposit(NearToken::from_near(10)),
            )
            .await?;
        Ok(())
    }

    async fn oc_set_updater_id(
        &self,
        target: &AccountId,
        new_updater_id: &AccountId,
    ) -> anyhow::Result<()> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("oc_set_updater_id")
                    .json_args(json!({"new_updater_id": new_updater_id}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;
        Ok(())
    }

    async fn oc_set_location(
        &self,
        target: &AccountId,
        location: WasmLocation,
    ) -> anyhow::Result<()> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("oc_set_location")
                    .json_args(json!({"location": location}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;
        Ok(())
    }
}

impl OutlayerProjectViewExt for Account {
    async fn oc_updater_id(&self) -> anyhow::Result<AccountId> {
        self.call_view_function_json("oc_updater_id", ()).await
    }

    async fn oc_wasm_hash(&self) -> anyhow::Result<[u8; 32]> {
        let hash: AsHex<[u8; 32]> = self.call_view_function_json("oc_wasm_hash", ()).await?;
        Ok(hash.into_inner())
    }

    async fn oc_wasm(&self) -> anyhow::Result<Option<Vec<u8>>> {
        let result: Option<defuse_outlayer_project::AsBase64<Vec<u8>>> =
            self.call_view_function_json("oc_wasm", ()).await?;
        Ok(result.map(|b| b.0))
    }

    async fn oc_location(&self) -> anyhow::Result<Option<WasmLocation>> {
        self.call_view_function_json("oc_location", ()).await
    }
}
