#![allow(clippy::option_env_unwrap)]
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{fs, path::Path};

pub const USE_OUT_DIR_VAR: &str = "DEFUSE_USE_OUT_DIR";

pub enum ReadWasmMode {
    WorkspaceRoot,
    BuildArtifact,
}

fn get_out_dir() -> PathBuf {
    let out_dir = std::env::var(USE_OUT_DIR_VAR)
        .ok()
        .or_else(|| option_env!("DEFUSE_OUT_DIR").map(str::to_owned))
        .unwrap_or_else(|| env!("OUT_DIR").to_owned());

    PathBuf::from(&out_dir)
}

pub fn read_wasm(mode: &ReadWasmMode, path: impl AsRef<Path>) -> Vec<u8> {
    let mut base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../");

    if matches!(mode, ReadWasmMode::BuildArtifact) {
        // if out dir path is absolute - base is ignored during join
        base = base.join(get_out_dir());
    }

    let path = fs::canonicalize(base.join(path))
        .unwrap_or_else(|e| panic!("Failed to canonicalize path: {e}"));

    println!("Reading WASM file at {}", path.display());

    fs::read(&path).unwrap_or_else(|e| panic!("Failed to read WASM file at {path:?}: {e}"))
}

pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    read_wasm(
        &ReadWasmMode::BuildArtifact,
        "multi_token_receiver_stub.wasm",
    )
});

pub static DEFUSE_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::BuildArtifact, "defuse.wasm"));
pub static DEFUSE_LEGACY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::WorkspaceRoot, "releases/previous.wasm"));

pub static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::BuildArtifact, "defuse_escrow_swap.wasm"));

pub static POA_FACTORY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::BuildArtifact, "defuse_poa_factory.wasm"));

pub static NON_FUNGIBLE_TOKEN_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    read_wasm(
        &ReadWasmMode::WorkspaceRoot,
        "releases/non-fungible-token.wasm",
    )
});

pub static WNEAR_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::WorkspaceRoot, "releases/wnear.wasm"));
