#![allow(async_fn_in_trait)]

pub mod convert;
pub mod extensions;
pub mod global_contract;
pub mod nep616;
pub mod outcome;

pub mod account;

use impl_tools::autoimpl;
use near_kit::{Near, sandbox::SandboxConfig};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::sync::atomic::{AtomicUsize, Ordering};

pub use anyhow;
pub use near_kit as kit;

use near_kit::NearToken;
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
        .create_subaccount(
            SUB_COUNTER.fetch_add(1, Ordering::Relaxed).to_string(),
            amount,
        )
        .await
}

// TODO: remove it after near kit update
#[serde_as]
#[derive(Serialize, Deserialize)]
#[autoimpl(Deref using self.0)]
pub struct U128(#[serde_as(as = "DisplayFromStr")] pub u128);

impl From<u128> for U128 {
    fn from(val: u128) -> Self {
        Self(val)
    }
}
