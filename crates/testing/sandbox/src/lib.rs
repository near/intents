#![allow(async_fn_in_trait)]

pub mod account;
pub mod extensions;
pub mod global_contract;
pub mod nep616;
pub mod outcome;

pub use near_kit as kit;
pub use near_kit_sandbox as sandbox;

use near_kit::{NearToken, SigningNear};
use near_kit_sandbox::Sandbox;
use rstest::fixture;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::account::Account;

#[fixture]
pub async fn root(#[default(NearToken::from_near(100_000))] amount: NearToken) -> SigningNear {
    static SUB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    Sandbox::builder()
        .shared()
        .await
        .expect("failed to start sandbox")
        .root_account()
        .create_subaccount(
            SUB_COUNTER.fetch_add(1, Ordering::Relaxed).to_string(),
            amount,
        )
        .await
}
