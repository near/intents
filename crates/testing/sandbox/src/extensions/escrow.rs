use anyhow::Result;
use defuse_escrow_swap::{Params, Storage};
use near_kit::{Final, Near};
use near_sdk::AccountId;

use crate::outcome::SuccessfulExecutionOutcome;

#[near_kit::contract]
pub trait Escrow {
    fn es_view(&self) -> Storage;

    #[call]
    fn es_close(&mut self, params: Params) -> bool;

    #[call]
    fn es_lost_found(&mut self, params: Params) -> bool;
}

pub trait EscrowExt {
    async fn es_close(
        &self,
        escrow_id: impl Into<AccountId>,
        params: Params,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn es_lost_found(
        &self,
        escrow_id: impl Into<AccountId>,
        params: Params,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl EscrowExt for Near {
    async fn es_close(
        &self,
        escrow_id: impl Into<AccountId>,
        params: Params,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(escrow_id.into())
            .add_action(Escrow::es_close(params))
            .wait_until(Final)
            .await?
            .try_into()
    }

    async fn es_lost_found(
        &self,
        escrow_id: impl Into<AccountId>,
        params: Params,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(escrow_id.into())
            .add_action(Escrow::es_lost_found(params))
            .wait_until(Final)
            .await?
            .try_into()
    }
}
