use anyhow::Result;
use defuse_escrow_swap::{Params, Storage};
use near_kit::{AccountIdRef, Final, Gas, Near};
use serde::{Deserialize, Serialize};

use crate::outcome::SuccessfulExecutionOutcome;

pub use defuse_escrow_swap as contract;

#[derive(Serialize, Deserialize)]
pub struct EsParams {
    pub params: Params,
}

#[near_kit::contract]
pub trait Escrow {
    fn es_view(&self) -> Storage;

    #[call]
    fn es_close(&mut self, params: EsParams) -> bool;

    #[call]
    fn es_lost_found(&mut self, params: EsParams) -> bool;
}

pub trait EscrowExt {
    async fn es_close(
        &self,
        escrow_id: impl AsRef<AccountIdRef>,
        params: Params,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn es_lost_found(
        &self,
        escrow_id: impl AsRef<AccountIdRef>,
        params: Params,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl EscrowExt for Near {
    async fn es_close(
        &self,
        escrow_id: impl AsRef<AccountIdRef>,
        params: Params,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(escrow_id.as_ref())
            .add_action(Escrow::es_close(EsParams { params }).gas(Gas::from_tgas(300)))
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn es_lost_found(
        &self,
        escrow_id: impl AsRef<AccountIdRef>,
        params: Params,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(escrow_id.as_ref())
            .add_action(Escrow::es_lost_found(EsParams { params }).gas(Gas::from_tgas(300)))
            .wait_until(Final)
            .await?
            .try_into()
    }
}
