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
    let out_dir = std::env::var(USE_OUT_DIR_VAR).expect("DEFUSE_USE_OUT_DIR not set");

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

    eprintln!("Reading WASM file at {}", path.display());

    fs::read(&path).unwrap_or_else(|e| panic!("Failed to read WASM file at {path:?}: {e}"))
}

#[cfg(feature = "defuse")]
pub static DEFUSE_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::BuildArtifact, "defuse.wasm"));
#[cfg(feature = "defuse")]
pub static DEFUSE_LEGACY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::WorkspaceRoot, "releases/previous.wasm"));

#[cfg(feature = "escrow")]
pub static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::BuildArtifact, "defuse_escrow_swap.wasm"));

#[cfg(feature = "poa")]
pub static POA_FACTORY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::BuildArtifact, "defuse_poa_factory.wasm"));

pub static NON_FUNGIBLE_TOKEN_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    read_wasm(
        &ReadWasmMode::WorkspaceRoot,
        "releases/non-fungible-token.wasm",
    )
});

pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    read_wasm(
        &ReadWasmMode::BuildArtifact,
        "multi_token_receiver_stub.wasm",
    )
});

pub static WNEAR_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::WorkspaceRoot, "releases/wnear.wasm"));

pub static DEPLOYER_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::BuildArtifact, "global_deployer.wasm"));

pub static DEPLOYER_WITH_USE_ME_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    read_wasm(
        &ReadWasmMode::BuildArtifact,
        "global_deployer_with_use_me.wasm",
    )
});
