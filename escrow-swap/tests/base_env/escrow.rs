use defuse_escrow_swap::{Params, Storage};
use defuse_sandbox::{Account, SigningAccount, tx::FnCallBuilder};
use near_sdk::{AccountId, serde_json::json};

pub trait EscrowViewExt {
    async fn view_escrow(&self) -> anyhow::Result<Storage>;
}

pub trait EscrowExt {
    // async fn deploy_escrow(
    //     &self,
    //     global_id: GlobalContractIdentifier,
    //     params: &Params,
    // ) -> TxResult<Account>;

    async fn close_escrow(&self, escrow: AccountId, params: Params) -> anyhow::Result<bool>;
}
impl EscrowExt for SigningAccount {
    // async fn deploy_escrow(
    //     &self,
    //     global_id: GlobalContractIdentifier,
    //     params: &Params,
    // ) -> TxResult<Account> {
    //     let init_args = json!({
    //         "params": params,
    //     });

    //     let account_id = Storage::new(params).unwrap().derive_account_id(self.id());

    //     self.tx(account_id.clone())
    //         .create_account()
    //         .use_global(global_id)
    //         .function_call_json::<()>(
    //             "escrow_init",
    //             init_args,
    //             Gas::from_tgas(10),
    //             NearToken::from_yoctonear(0),
    //         )
    //         .no_result()
    //         .await?;

    //     Ok(Account::new(account_id, self.network_config().clone()))
    // }

    async fn close_escrow(&self, escrow: AccountId, params: Params) -> anyhow::Result<bool> {
        self.tx(escrow.clone())
            .function_call(FnCallBuilder::new("escrow_close").json_args(json!({
                "params": params,
            })))
            .await
            .and_then(|r| r.json::<bool>().map_err(Into::into))
    }
}

impl EscrowViewExt for Account {
    async fn view_escrow(&self) -> anyhow::Result<Storage> {
        self.call_view_function_json("escrow_view", ()).await
    }
}
