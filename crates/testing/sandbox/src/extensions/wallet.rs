use anyhow::Result;
use defuse_wallet::{Request, signature::Deadline, signature::RequestMessage};
use near_kit::{AccountId, AccountIdRef, Final, Near, NearToken, StateInit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{extensions::DEFAULT_GAS, outcome::SuccessfulExecutionOutcome};

pub use defuse_wallet as contract;
pub use defuse_wallet_sdk as sdk;

#[derive(Serialize, Deserialize)]
pub struct ExecuteSignedArgs {
    pub msg: RequestMessage,
    pub proof: String,
}

#[derive(Serialize, Deserialize)]
pub struct ExecuteExtensionArgs {
    pub request: Request,
}

#[near_kit::contract]
pub trait Wallet {
    #[call]
    fn w_execute_signed(&mut self, args: ExecuteSignedArgs) -> bool;

    #[call]
    fn w_execute_extension(&mut self, args: ExecuteExtensionArgs) -> bool;

    fn w_subwallet_id(&self) -> u32;
    fn w_is_signature_allowed(&self) -> bool;
    fn w_public_key(&self) -> String;
    fn w_is_extension_enabled(&self, account_id: AccountId) -> bool;
    fn w_extensions(&self) -> BTreeSet<AccountId>;
    fn w_timeout_secs(&self) -> u64;
    fn w_last_cleaned_at(&self) -> Deadline;
}

pub trait WalletExt {
    async fn w_execute_signed(
        &self,
        wallet_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        msg: RequestMessage,
        proof: String,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome>;

    async fn w_execute_extension(
        &self,
        wallet_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        request: Request,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome>;
}

impl WalletExt for Near {
    async fn w_execute_signed(
        &self,
        wallet_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        msg: RequestMessage,
        proof: String,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome> {
        let mut tx = self.transaction(wallet_id.as_ref());

        if let Some(state_init) = state_init.into() {
            tx = tx.state_init(state_init, NearToken::ZERO);
        }

        tx.add_action(
            Wallet::w_execute_signed(ExecuteSignedArgs { msg, proof })
                .deposit(deposit)
                .gas(DEFAULT_GAS),
        )
        .wait_until(Final)
        .await?
        .try_into()
    }

    async fn w_execute_extension(
        &self,
        wallet_id: impl AsRef<AccountIdRef>,
        state_init: impl Into<Option<StateInit>>,
        request: Request,
        deposit: NearToken,
    ) -> Result<SuccessfulExecutionOutcome> {
        let mut tx = self.transaction(wallet_id.as_ref());

        if let Some(state_init) = state_init.into() {
            tx = tx.state_init(state_init, NearToken::ZERO);
        }
        tx.add_action(
            Wallet::w_execute_extension(ExecuteExtensionArgs { request })
                .deposit(deposit)
                .gas(DEFAULT_GAS),
        )
        .wait_until(Final)
        .await?
        .try_into()
    }
}
