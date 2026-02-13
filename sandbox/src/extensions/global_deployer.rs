use crate::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use defuse_global_deployer::State as DeployerState;
use near_api::types::transaction::result::ExecutionSuccess;
use near_sdk::{
    AccountId, GlobalContractId, NearToken,
    serde_json::json,
    state_init::{StateInit, StateInitV1},
};

#[allow(async_fn_in_trait)]
pub trait DeployerExt {
    async fn deploy_instance(
        &self,
        deployer_code_hash_id: GlobalContractId,
        state: DeployerState,
    ) -> anyhow::Result<Account>;

    async fn gd_deploy(
        &self,
        target: &AccountId,
        old_hash: [u8; 32],
        new_code: &[u8],
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn gd_transfer_ownership(
        &self,
        target: &AccountId,
        new_owner: &AccountId,
    ) -> anyhow::Result<ExecutionSuccess>;
}

#[allow(async_fn_in_trait)]
pub trait DeployerViewExt {
    async fn gd_owner_id(&self) -> anyhow::Result<AccountId>;
    async fn gd_index(&self) -> anyhow::Result<u32>;
    async fn gd_code_hash(&self) -> anyhow::Result<[u8; 32]>;
}

impl DeployerExt for SigningAccount {
    async fn deploy_instance(
        &self,
        global_contract_id: GlobalContractId,
        state: DeployerState,
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

    async fn gd_deploy(
        &self,
        target: &AccountId,
        old_hash: [u8; 32],
        new_code: &[u8],
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("gd_deploy")
                    .borsh_args(&(old_hash, new_code))
                    .with_deposit(NearToken::from_near(50)),
            )
            .await
    }

    async fn gd_transfer_ownership(
        &self,
        target: &AccountId,
        new_owner: &AccountId,
    ) -> anyhow::Result<ExecutionSuccess> {
        self.tx(target)
            .function_call(
                FnCallBuilder::new("gd_transfer_ownership")
                    .json_args(json!({"receiver_id": new_owner.clone()}))
                    .with_deposit(NearToken::from_yoctonear(1)),
            )
            .await
    }
}

impl DeployerViewExt for Account {
    async fn gd_owner_id(&self) -> anyhow::Result<AccountId> {
        self.call_view_function_json("gd_owner_id", ()).await
    }

    async fn gd_index(&self) -> anyhow::Result<u32> {
        self.call_view_function_json("gd_index", ()).await
    }

    async fn gd_code_hash(&self) -> anyhow::Result<[u8; 32]> {
        self.call_view_function_json("gd_code_hash", ()).await
    }
}
