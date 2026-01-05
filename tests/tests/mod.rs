mod defuse;
mod utils;

use defuse_sandbox::read_wasm;
use std::sync::LazyLock;

pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/multi-token-receiver-stub/multi_token_receiver_stub"));
