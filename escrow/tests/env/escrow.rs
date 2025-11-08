use defuse_escrow::{ContractState, FixedParams, Params, Storage};
use defuse_sandbox::{
    Account, SigningAccount, TxResult, api::types::transaction::actions::GlobalContractIdentifier,
};
use near_sdk::{AccountId, Gas, NearToken, json_types::U128, serde_json::json};

pub trait EscrowViewExt {
    async fn view_escrow(&self) -> anyhow::Result<ContractState>;
}

pub trait EscrowExt {
    async fn deploy_escrow(
        &self,
        global_id: GlobalContractIdentifier,
        fixed: &FixedParams,
        params: Params,
    ) -> TxResult<Account>;

    async fn close_escrow(&self, escrow: AccountId, fixed_params: FixedParams) -> TxResult<u128>;
}
impl EscrowExt for SigningAccount {
    async fn deploy_escrow(
        &self,
        global_id: GlobalContractIdentifier,
        fixed: &FixedParams,
        params: Params,
    ) -> TxResult<Account> {
        let init_args = json!({
            "fixed": fixed,
            "params": params,
        });

        let account_id = Storage::new(fixed, params).derive_account_id(self.id());

        self.tx(account_id.clone())
            .create_account()
            .use_global(global_id)
            .function_call_json::<()>(
                "new",
                init_args,
                Gas::from_tgas(10),
                NearToken::from_yoctonear(0),
            )
            .no_result()
            .await?;

        Ok(Account::new(account_id, self.network_config().clone()))
    }

    async fn close_escrow(&self, escrow: AccountId, fixed_params: FixedParams) -> TxResult<u128> {
        self.tx(escrow.clone())
            .function_call_json::<U128>(
                "close",
                json!({
                    "fixed_params": fixed_params,
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(0),
            )
            .await
            .map(|a| a.0)
    }
}

impl EscrowViewExt for Account {
    async fn view_escrow(&self) -> anyhow::Result<ContractState> {
        self.call_function_json("view", ()).await
    }
}
