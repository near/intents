#![allow(async_fn_in_trait, dead_code)]

mod env;
mod fees;
mod partial_fills;
mod swaps;

use std::sync::LazyLock;

use defuse_escrow_swap::{Params, Storage};
use defuse_sandbox::{Account, SigningAccount, read_wasm, tx::FnCallBuilder};
use near_sdk::AccountId;
use serde_json::json;

static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("res/defuse_escrow_swap"));

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

pub trait EscrowExtView {
    async fn es_view(&self) -> anyhow::Result<Storage>;
}

impl EscrowExtView for Account {
    async fn es_view(&self) -> anyhow::Result<Storage> {
        self.call_view_function_json("es_view", ()).await
    }
}
