use std::{collections::BTreeSet, time::Duration};

use defuse_wallet::{Request, signature::Deadline, signature::RequestMessage};
use near_api::types::transaction::result::ExecutionSuccess;
use near_sdk::{AccountId, AccountIdRef, NearToken, serde_json::json, state_init::StateInit};

use crate::{Account, FnCallBuilder, SigningAccount};

#[allow(async_fn_in_trait)]
pub trait WalletExt {
    async fn w_execute_signed(
        &self,
        wallet_id: impl Into<AccountId>,
        state_init: impl Into<Option<StateInit>>,
        msg: RequestMessage,
        proof: String,
        deposit: NearToken,
    ) -> anyhow::Result<ExecutionSuccess>;

    async fn w_execute_extension(
        &self,
        wallet_id: impl Into<AccountId>,
        state_init: impl Into<Option<StateInit>>,
        request: Request,
        deposit: NearToken,
    ) -> anyhow::Result<ExecutionSuccess>;
}

#[allow(async_fn_in_trait)]
pub trait WalletViewExt {
    async fn w_subwallet_id(&self) -> anyhow::Result<u32>;
    async fn w_is_signature_allowed(&self) -> anyhow::Result<bool>;
    async fn w_public_key(&self) -> anyhow::Result<String>;
    async fn w_is_extension_enabled(
        &self,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<bool>;
    async fn w_extensions(&self) -> anyhow::Result<BTreeSet<AccountId>>;
    async fn w_timeout(&self) -> anyhow::Result<Duration>;
    async fn w_last_cleaned_at(&self) -> anyhow::Result<Deadline>;
}

impl WalletExt for SigningAccount {
    async fn w_execute_signed(
        &self,
        wallet_id: impl Into<AccountId>,
        state_init: impl Into<Option<StateInit>>,
        msg: RequestMessage,
        proof: String,
        deposit: NearToken,
    ) -> anyhow::Result<ExecutionSuccess> {
        let mut tx = self.tx(wallet_id);
        if let Some(state_init) = state_init.into() {
            tx = tx.state_init(state_init, NearToken::ZERO);
        }
        tx.function_call(
            FnCallBuilder::new("w_execute_signed")
                .json_args(json!({
                    "msg": msg,
                    "proof": proof,
                }))
                .with_deposit(deposit),
        )
        .await
    }

    async fn w_execute_extension(
        &self,
        wallet_id: impl Into<AccountId>,
        state_init: impl Into<Option<StateInit>>,
        request: Request,
        deposit: NearToken,
    ) -> anyhow::Result<ExecutionSuccess> {
        let mut tx = self.tx(wallet_id);
        if let Some(state_init) = state_init.into() {
            tx = tx.state_init(state_init, NearToken::ZERO);
        }
        tx.function_call(
            FnCallBuilder::new("w_execute_extension")
                .json_args(json!({
                    "request": request,
                }))
                .with_deposit(deposit),
        )
        .await
    }
}

impl WalletViewExt for Account {
    async fn w_subwallet_id(&self) -> anyhow::Result<u32> {
        self.call_view_function_json("w_subwallet_id", ()).await
    }

    async fn w_is_signature_allowed(&self) -> anyhow::Result<bool> {
        self.call_view_function_json("w_is_signature_allowed", ())
            .await
    }

    async fn w_public_key(&self) -> anyhow::Result<String> {
        self.call_view_function_json("w_public_key", ()).await
    }

    async fn w_is_extension_enabled(
        &self,
        account_id: impl AsRef<AccountIdRef>,
    ) -> anyhow::Result<bool> {
        self.call_view_function_json(
            "w_is_extension_enabled",
            json!({
                "account_id": account_id.as_ref(),
            }),
        )
        .await
    }

    async fn w_extensions(&self) -> anyhow::Result<BTreeSet<AccountId>> {
        self.call_view_function_json("w_extensions", ()).await
    }

    async fn w_timeout(&self) -> anyhow::Result<Duration> {
        self.call_view_function_json("w_timeout_secs", ())
            .await
            .map(Duration::from_secs)
    }

    async fn w_last_cleaned_at(&self) -> anyhow::Result<Deadline> {
        self.call_view_function_json("w_last_cleaned_at", ()).await
    }
}
