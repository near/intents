use defuse_escrow_swap::{Params, Storage};
use defuse_sandbox::{
    Account, SigningAccount, TxResult, api::types::transaction::actions::GlobalContractIdentifier,
};
use near_sdk::{AccountId, Gas, NearToken, serde_json::json};

pub trait EscrowViewExt {
    async fn view_escrow(&self) -> anyhow::Result<Storage>;
}

pub trait EscrowExt {
    async fn deploy_escrow(
        &self,
        global_id: GlobalContractIdentifier,
        params: &Params,
    ) -> TxResult<Account>;

    async fn close_escrow(&self, escrow: AccountId, params: Params) -> TxResult<bool>;
}
impl EscrowExt for SigningAccount {
    async fn deploy_escrow(
        &self,
        global_id: GlobalContractIdentifier,
        params: &Params,
    ) -> TxResult<Account> {
        let init_args = json!({
            "params": params,
        });

        let account_id = Storage::new(params).unwrap().derive_account_id(self.id());

        self.tx(account_id.clone())
            .create_account()
            .use_global(global_id)
            .function_call_json::<()>(
                "escrow_init",
                init_args,
                Gas::from_tgas(10),
                NearToken::from_yoctonear(0),
            )
            .no_result()
            .await?;

        Ok(Account::new(account_id, self.network_config().clone()))
    }

    async fn close_escrow(&self, escrow: AccountId, params: Params) -> TxResult<bool> {
        self.tx(escrow.clone())
            .function_call_json(
                "escrow_close",
                json!({
                    "params": params,
                }),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(0),
            )
            .await
    }
}

impl EscrowViewExt for Account {
    async fn view_escrow(&self) -> anyhow::Result<Storage> {
        self.call_function_json("escrow_view", ()).await
    }
}
