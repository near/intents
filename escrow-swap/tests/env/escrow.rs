use defuse_escrow_swap::{Params, Storage};
use defuse_sandbox::{Account, FnCallBuilder, SigningAccount};
use near_sdk::{AccountId, Gas, serde_json::json};

pub trait EscrowViewExt {
    async fn view_escrow(&self) -> anyhow::Result<Storage>;
}

pub trait EscrowExt {
    async fn close_escrow(&self, escrow: AccountId, params: Params) -> anyhow::Result<bool>;
}

impl EscrowExt for SigningAccount {
    async fn close_escrow(&self, escrow: AccountId, params: Params) -> anyhow::Result<bool> {
        self.tx(escrow.clone())
            .function_call(
                FnCallBuilder::new("escrow_close")
                    .json_args(json!({
                        "params": params,
                    }))
                    .with_gas(Gas::from_tgas(300)),
            )
            .await?
            .json()
            .map_err(Into::into)
    }
}

impl EscrowViewExt for Account {
    async fn view_escrow(&self) -> anyhow::Result<Storage> {
        self.call_view_function_json("escrow_view", ()).await
    }
}
