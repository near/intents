#![allow(clippy::option_env_unwrap)]
use std::sync::LazyLock;
use std::{fs, path::Path};

pub const DEFUSE_RELEASE_DIR: &str = "releases";

pub enum ReadWasmMode {
    FromReleases,
    FromOutdir,
}

pub fn read_wasm(mode: &ReadWasmMode, path: impl AsRef<Path>) -> Vec<u8> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../");
    let dir = match mode {
        ReadWasmMode::FromOutdir => {
            base.join(option_env!("DEFUSE_OUT_DIR").expect("Out dir should be set"))
        }
        ReadWasmMode::FromReleases => base.join(DEFUSE_RELEASE_DIR),
    };

    let path = fs::canonicalize(dir.join(path))
        .unwrap_or_else(|e| panic!("Failed to canonicalize path: {e}"));

    println!("Reading WASM file at {}", path.display());

    fs::read(&path).unwrap_or_else(|e| panic!("Failed to read WASM file at {path:?}: {e}"))
}

pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::FromOutdir, "multi_token_receiver_stub.wasm"));

pub static DEFUSE_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::FromOutdir, "defuse.wasm"));
pub static DEFUSE_LEGACY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::FromReleases, "previous.wasm"));

pub static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::FromOutdir, "defuse_escrow_swap.wasm"));

pub static POA_FACTORY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::FromOutdir, "defuse_poa_factory.wasm"));

pub static NON_FUNGIBLE_TOKEN_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::FromReleases, "non-fungible-token.wasm"));

pub static WNEAR_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::FromReleases, "wnear.wasm"));
