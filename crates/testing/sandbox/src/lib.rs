#![allow(async_fn_in_trait)]

pub mod account;
pub mod extensions;
pub mod global_contract;
mod helpers;
pub mod nep616;
pub mod outcome;

pub use self::helpers::*;

pub use near_kit as kit;

use near_kit::{Near, NearToken, sandbox::SandboxConfig};
use rstest::fixture;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::account::Account;

#[fixture]
pub async fn root(#[default(NearToken::from_near(100_000))] amount: NearToken) -> Near {
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
