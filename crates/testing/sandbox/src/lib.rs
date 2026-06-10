#![allow(async_fn_in_trait)]

pub mod extensions;
pub mod nep616;
pub mod outcome;
pub mod state_init;

pub mod account;

use near_kit::{Near, sandbox::SandboxConfig};
use std::sync::atomic::{AtomicUsize, Ordering};

pub use anyhow;
pub use near_kit as kit;

use near_sdk::NearToken;
use rstest::fixture;
use tracing::instrument;

use crate::account::Account;

#[fixture]
#[instrument]
pub async fn root(#[default(NearToken::from_near(100_000))] amount: NearToken) -> Near {
    // TODO: do we really need this counter?
    static SUB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    SandboxConfig::shared()
        .await
        .client()
        .generate_subaccount(
            SUB_COUNTER.fetch_add(1, Ordering::Relaxed).to_string(),
            amount,
        )
        .await
}
