use std::sync::LazyLock;
use std::{fs, path::Path};

pub fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../")
            .join(name)
            .with_extension("wasm"),
    )
    .unwrap_or_else(|e| panic!("Failed to canonicalize path: {e}"));

    println!("Reading WASM file at {filename:?}");

    fs::read(&filename).unwrap_or_else(|e| panic!("Failed to read WASM file at {filename:?}: {e}"))
}

#[allow(dead_code)]
pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/multi-token-receiver-stub/multi_token_receiver_stub"));

pub static DEFUSE_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("res/defuse"));
pub static DEFUSE_LEGACY_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("releases/previous"));

pub static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("res/defuse_escrow_swap"));

pub static POA_FACTORY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("releases/defuse_poa_factory"));

pub static NON_FUNGIBLE_TOKEN_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("releases/non-fungible-token"));

pub static WNEAR_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("releases/wnear"));
