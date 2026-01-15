#[cfg(feature = "defuse")]
mod defuse;

#[cfg(feature = "poa")]
mod poa;

#[cfg(feature = "escrow")]
mod escrow;

mod utils;

use defuse_sandbox::read_wasm;
use std::sync::LazyLock;

#[allow(dead_code)]
pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/multi-token-receiver-stub/multi_token_receiver_stub"));
