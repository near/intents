use anyhow::Result;
use defuse_escrow_swap::{Params, Storage};
use near_kit::{AccountIdRef, Gas, SigningNear, rpc::CheckedOutcome};

pub use defuse_escrow_swap as contract;

#[near_kit::contract]
pub trait Escrow {
    /// See [`es_view()`](defuse_escrow_swap::Escrow::es_view) method.
    fn es_view(&self) -> Storage;

    /// See [`es_close()`](defuse_escrow_swap::Escrow::es_close) method.
    #[call]
    fn es_close(&mut self, params: &Params);

    /// See [`es_lost_found()`](defuse_escrow_swap::Escrow::es_lost_found) method.
    #[call]
    fn es_lost_found(&mut self, params: &Params);
}

pub trait EscrowExt {
    async fn es_close(
        &self,
        escrow_id: impl AsRef<AccountIdRef>,
        params: Params,
    ) -> Result<CheckedOutcome>;

    async fn es_lost_found(
        &self,
        escrow_id: impl AsRef<AccountIdRef>,
        params: Params,
    ) -> Result<CheckedOutcome>;
}

impl EscrowExt for SigningNear {
    async fn es_close(
        &self,
        escrow_id: impl AsRef<AccountIdRef>,
        params: Params,
    ) -> Result<CheckedOutcome> {
        self.transaction(escrow_id.as_ref())
            .add_action(Escrow::es_close(&params).gas(Gas::from_tgas(300)))
            .await?
            .checked()
            .map_err(Into::into)
    }

    async fn es_lost_found(
        &self,
        escrow_id: impl AsRef<AccountIdRef>,
        params: Params,
    ) -> Result<CheckedOutcome> {
        self.transaction(escrow_id.as_ref())
            .add_action(Escrow::es_lost_found(&params).gas(Gas::from_tgas(300)))
            .await?
            .checked()
            .map_err(Into::into)
    }
}
