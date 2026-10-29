use anyhow::Result;
use defuse_wallet_sdk::{Request, RequestMessage, client::WalletContract};
use near_kit::{
    AccountIdRef, Gas, NearToken, SigningNear,
    protocol::{Action, StateInit, derive_deterministic_account_id},
};

pub use defuse_wallet_sdk as sdk;

use crate::outcome::SuccessfulExecutionOutcome;

pub trait WalletExt {
    async fn w_execute_signed(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        msg: &RequestMessage,
        proof: impl AsRef<str>,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn w_execute_extension(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        request: &Request,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl WalletExt for SigningNear {
    async fn w_execute_signed(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        msg: &RequestMessage,
        proof: impl AsRef<str>,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome> {
        let state_init = state_init.into();
        // a state_init is executed on the account derived from it
        let mut tx = self.transaction(state_init.as_ref().map_or_else(
            || contract_id.as_ref().into(),
            derive_deterministic_account_id,
        ));

        if let Some(state_init) = state_init {
            tx = tx.add_action(Action::state_init(state_init, NearToken::ZERO));
        }

        tx.add_action(
            WalletContract::w_execute_signed((msg, proof.as_ref()).into())
                .deposit(deposit)
                .gas(Gas::from_tgas(300)),
        )
        .await?
        .try_into()
    }

    async fn w_execute_extension(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        request: &Request,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome> {
        let state_init = state_init.into();
        // a state_init is executed on the account derived from it
        let mut tx = self.transaction(state_init.as_ref().map_or_else(
            || contract_id.as_ref().into(),
            derive_deterministic_account_id,
        ));

        if let Some(state_init) = state_init {
            tx = tx.add_action(Action::state_init(state_init, NearToken::ZERO));
        }
        tx.add_action(
            WalletContract::w_execute_extension(request.into())
                .deposit(deposit)
                .gas(Gas::from_tgas(300)),
        )
        .await?
        .try_into()
    }
}
