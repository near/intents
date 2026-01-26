pub use defuse_escrow_swap as contract;

use defuse_sandbox::{Account, SigningAccount, anyhow, tx::FnCallBuilder};
use near_sdk::{AccountId, serde_json::json};

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
