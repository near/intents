#![allow(async_fn_in_trait, dead_code)]

mod helpers;
mod fees;
mod partial_fills;
mod swaps;

use std::sync::LazyLock;

use crate::env::read_wasm;

// Re-export from extensions to avoid duplication
pub use crate::extensions::escrow::EscrowExt;

static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("res/defuse_escrow_swap"));
