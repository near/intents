#![allow(clippy::option_env_unwrap)]
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{fs, path::Path};

pub enum ReadWasmMode {
    RelativePath,
    FromManifestDir,
}

pub fn read_wasm(mode: &ReadWasmMode, path: impl AsRef<Path>) -> Vec<u8> {
    let path = match mode {
        ReadWasmMode::RelativePath => PathBuf::from(path.as_ref()),
        ReadWasmMode::FromManifestDir => fs::canonicalize(
            Path::new(option_env!("CARGO_MANIFEST_DIR").expect("Missing env var"))
                .join("../")
                .join(path),
        )
        .unwrap_or_else(|e| panic!("Failed to canonicalize path: {e}")),
    };

    println!("Reading WASM file at {}", path.display());

    fs::read(&path).unwrap_or_else(|e| panic!("Failed to read WASM file at {path:?}: {e}"))
}

pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    read_wasm(
        &ReadWasmMode::RelativePath,
        option_env!("DEFUSE_MULTI_TOKEN_RECEIVER_STUB_WASM").expect("Missing env var"),
    )
});

pub static DEFUSE_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    read_wasm(
        &ReadWasmMode::RelativePath,
        option_env!("DEFUSE_WASM").expect("Missing env var"),
    )
});
pub static DEFUSE_LEGACY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::FromManifestDir, "releases/previous.wasm"));

pub static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    read_wasm(
        &ReadWasmMode::RelativePath,
        option_env!("DEFUSE_ESCROW_SWAP_WASM").expect("Missing env var"),
    )
});

pub static POA_FACTORY_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    read_wasm(
        &ReadWasmMode::RelativePath,
        option_env!("DEFUSE_POA_FACTORY_WASM").expect("Missing env var"),
    )
});

pub static NON_FUNGIBLE_TOKEN_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    read_wasm(
        &ReadWasmMode::FromManifestDir,
        "releases/non-fungible-token.wasm",
    )
});

pub static WNEAR_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(&ReadWasmMode::FromManifestDir, "releases/wnear.wasm"));
