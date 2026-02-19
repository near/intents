use crate::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use defuse_deployer_hash_proxy::State as HashProxyState;
use near_api::types::transaction::result::ExecutionSuccess;
use near_sdk::{
    GlobalContractId, NearToken,
    serde_json::json,
    state_init::{StateInit, StateInitV1},
};

#[allow(async_fn_in_trait)]
pub trait DeployerHashProxyExt {
    async fn deploy_hash_proxy_instance(
        &self,
        global_contract_id: GlobalContractId,
        state: HashProxyState,
    ) -> anyhow::Result<Account>;

    async fn deploy_and_approve(
        &self,
        global_contract_id: GlobalContractId,
        state: HashProxyState,
    ) -> anyhow::Result<Account>;

    async fn hp_approve(
        &self,
        target: &near_sdk::AccountId,
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn hp_exec(
        &self,
        target: &near_sdk::AccountId,
        new_code: &[u8],
    ) -> anyhow::Result<ExecutionSuccess>;
}

#[allow(async_fn_in_trait)]
pub trait DeployerHashProxyViewExt {
    async fn hp_state(&self) -> anyhow::Result<HashProxyState>;
}

impl DeployerHashProxyExt for SigningAccount {
    async fn deploy_hash_proxy_instance(
        &self,
        global_contract_id: GlobalContractId,
        state: HashProxyState,
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

    async fn deploy_and_approve(
        &self,
        global_contract_id: GlobalContractId,
        state: HashProxyState,
    ) -> anyhow::Result<Account> {
        let state_init = StateInit::V1(StateInitV1 {
            code: global_contract_id,
            data: state.state_init(),
        });
        let account_id = state_init.derive_account_id();

        self.tx(account_id.clone())
            .state_init(state_init, NearToken::ZERO)
            .function_call(
                FnCallBuilder::new("hp_approve")
                    .json_args(json!({}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await?;

        Ok(Account::new(account_id, self.network_config().clone()))
    }

    async fn hp_approve(
        &self,
        target: &near_sdk::AccountId,
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("hp_approve")
                    .json_args(json!({}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
    }

    async fn hp_exec(
        &self,
        //TODO: account id ref
        target: &near_sdk::AccountId,
        new_code: &[u8],
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("hp_exec")
                    .borsh_args(&new_code)
                    .with_deposit(NearToken::from_near(50)),
            )
            .await
    }
}

impl DeployerHashProxyViewExt for Account {
    async fn hp_state(&self) -> anyhow::Result<HashProxyState> {
        self.call_view_function_json("hp_state", ()).await
    }
}
