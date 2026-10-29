use anyhow::Result;
use defuse_escrow_swap::{Params, Storage};
use near_kit::{AccountIdRef, Gas, SigningNear};

use crate::outcome::SuccessfulExecutionOutcome;

pub use defuse_escrow_swap as contract;

#[near_kit::contract]
pub trait Escrow {
    fn es_view(&self) -> Storage;

    /// Returns `bool` (decode it from the outcome).
    #[call]
    fn es_close(&mut self, params: Params);

    /// Returns `bool` (decode it from the outcome).
    #[call]
    fn es_lost_found(&mut self, params: Params);
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

impl EscrowExt for SigningNear {
    async fn es_close(
        &self,
        escrow_id: impl AsRef<AccountIdRef>,
        params: Params,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(escrow_id.as_ref())
            .add_action(Escrow::es_close(params).gas(Gas::from_tgas(300)))
            .await?
            .try_into()
    }

    async fn es_lost_found(
        &self,
        escrow_id: impl AsRef<AccountIdRef>,
        params: Params,
    ) -> Result<SuccessfulExecutionOutcome> {
        self.transaction(escrow_id.as_ref())
            .add_action(Escrow::es_lost_found(params).gas(Gas::from_tgas(300)))
            .await?
            .try_into()
    }
}
