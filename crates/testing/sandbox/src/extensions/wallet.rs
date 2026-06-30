use std::borrow::Cow;

use anyhow::Result;
use defuse_wallet_sdk::{Request, RequestMessage};
use near_kit::{AccountIdRef, Final, Gas, Near, NearToken, StateInit};

pub use defuse_wallet_client::*;
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

impl WalletExt for Near {
    async fn w_execute_signed(
        &self,
        contract_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        msg: &RequestMessage,
        proof: impl AsRef<str>,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome> {
        let mut tx = self.transaction(contract_id.as_ref());

        if let Some(state_init) = state_init.into() {
            tx = tx.state_init(state_init, NearToken::ZERO);
        }

        tx.add_action(
            Wallet::w_execute_signed(WExecuteSignedArgs {
                msg: Cow::Borrowed(msg),
                proof: proof.as_ref().into(),
            })
            .deposit(deposit)
            .gas(Gas::from_tgas(300)),
        )
        .wait_until(Final)
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
        let mut tx = self.transaction(contract_id.as_ref());

        if let Some(state_init) = state_init.into() {
            tx = tx.state_init(state_init, NearToken::ZERO);
        }
        tx.add_action(
            Wallet::w_execute_extension(WExecuteExtensionArgs {
                request: Cow::Borrowed(request),
            })
            .deposit(deposit)
            .gas(Gas::from_tgas(300)),
        )
        .wait_until(Final)
        .await?
        .try_into()
    }
}
