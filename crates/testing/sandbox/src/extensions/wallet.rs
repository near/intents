use anyhow::Result;
use defuse_wallet_sdk::{Request, RequestMessage, client::WalletContract};
use near_kit::{
    AccountIdRef, Gas, NearToken, SigningNear,
    protocol::{Action, StateInit},
};

use crate::outcome::{CheckedOutcome, IntoChecked};

pub use defuse_wallet_sdk as sdk;

pub trait WalletExt {
    async fn w_execute_signed(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        msg: &RequestMessage,
        proof: impl AsRef<str>,
        deposit: NearToken,
    ) -> Result<CheckedOutcome>;

    async fn w_execute_extension(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        request: &Request,
        deposit: NearToken,
    ) -> Result<CheckedOutcome>;
}

impl WalletExt for SigningNear {
    async fn w_execute_signed(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        msg: &RequestMessage,
        proof: impl AsRef<str>,
        deposit: NearToken,
    ) -> Result<CheckedOutcome> {
        let mut tx = self.transaction(contract_id.as_ref());

        if let Some(state_init) = state_init.into() {
            tx = tx.add_action(Action::state_init(state_init, NearToken::ZERO));
        }

        tx.add_action(
            WalletContract::w_execute_signed(msg, proof.as_ref())
                .deposit(deposit)
                .gas(Gas::from_tgas(300)),
        )
        .await
        .into_checked()
    }

    async fn w_execute_extension(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        request: &Request,
        deposit: NearToken,
    ) -> Result<CheckedOutcome> {
        let mut tx = self.transaction(contract_id.as_ref());

        if let Some(state_init) = state_init.into() {
            tx = tx.add_action(Action::state_init(state_init, NearToken::ZERO));
        }
        tx.add_action(
            WalletContract::w_execute_extension(request)
                .deposit(deposit)
                .gas(Gas::from_tgas(300)),
        )
        .await
        .into_checked()
    }
}
