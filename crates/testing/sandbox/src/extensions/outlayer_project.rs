use crate::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use defuse_outlayer_project::{State as OutlayerState, WasmLocation};
use defuse_serde_utils::hex::AsHex;
use near_api::types::transaction::result::ExecutionSuccess;
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

    /// Deploy a new `outlayer-project` instance and upload its WASM in a single transaction.
    async fn deploy_outlayer_project_with_inline_wasm(
        &self,
        global_contract_id: GlobalContractId,
        wasm: &[u8],
    ) -> anyhow::Result<(Account, ExecutionSuccess)>;

    async fn op_approve(
        &self,
        target: &AccountId,
        new_hash: [u8; 32],
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn op_upload_wasm(
        &self,
        target: &AccountId,
        code: &[u8],
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn op_set_updater_id(
        &self,
        target: &AccountId,
        new_updater_id: &AccountId,
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn op_set_location(
        &self,
        target: &AccountId,
        location: WasmLocation,
    ) -> anyhow::Result<ExecutionSuccess>;
}

#[allow(async_fn_in_trait)]
pub trait OutlayerProjectViewExt {
    async fn op_updater_id(&self) -> anyhow::Result<AccountId>;
    async fn op_wasm_hash(&self) -> anyhow::Result<[u8; 32]>;
    async fn op_wasm(&self) -> anyhow::Result<Option<Vec<u8>>>;
    async fn op_location(&self) -> anyhow::Result<Option<WasmLocation>>;
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

    async fn deploy_outlayer_project_with_inline_wasm(
        &self,
        global_contract_id: GlobalContractId,
        wasm: &[u8],
    ) -> anyhow::Result<(Account, ExecutionSuccess)> {
        use near_sdk::{Gas, env::sha256_array};
        let wasm_hash = sha256_array(wasm);

        let state_init = StateInit::V1(StateInitV1 {
            code: global_contract_id,
            data: OutlayerState::new(self.id().clone())
                .pre_approve(wasm_hash)
                .state_init(),
        });

        let account_id = state_init.derive_account_id();
        let result = self
            .tx(account_id.clone())
            .state_init(state_init, NearToken::ZERO)
            .function_call(
                FnCallBuilder::new("op_upload_wasm")
                    .raw_args(wasm.to_vec())
                    .with_deposit(NearToken::from_near(10))
                    .with_gas(Gas::from_tgas(290)),
            )
            .await?;
        Ok((
            Account::new(account_id, self.network_config().clone()),
            result,
        ))
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

    async fn op_upload_wasm(
        &self,
        target: &AccountId,
        code: &[u8],
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("op_upload_wasm")
                    .raw_args(code.to_vec())
                    .with_deposit(NearToken::from_near(10)),
            )
            .await
    }

    async fn op_set_updater_id(
        &self,
        target: &AccountId,
        new_updater_id: &AccountId,
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("op_set_updater_id")
                    .json_args(json!({"new_updater_id": new_updater_id}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
    }

    async fn op_set_location(
        &self,
        target: &AccountId,
        location: WasmLocation,
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("op_set_location")
                    .json_args(json!({"location": location}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
    }
}

impl OutlayerProjectViewExt for Account {
    async fn op_updater_id(&self) -> anyhow::Result<AccountId> {
        self.call_view_function_json("op_updater_id", ()).await
    }

    async fn op_wasm_hash(&self) -> anyhow::Result<[u8; 32]> {
        let hash: AsHex<[u8; 32]> = self.call_view_function_json("op_wasm_hash", ()).await?;
        Ok(hash.into_inner())
    }

    async fn op_wasm(&self) -> anyhow::Result<Option<Vec<u8>>> {
        let result: Option<defuse_outlayer_project::AsBase64<Vec<u8>>> =
            self.call_view_function_json("op_wasm", ()).await?;
        Ok(result.map(|b| b.0))
    }

    async fn op_location(&self) -> anyhow::Result<Option<WasmLocation>> {
        self.call_view_function_json("op_location", ()).await
    }
}
