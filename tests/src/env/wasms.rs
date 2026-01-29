use std::path::PathBuf;
use std::sync::LazyLock;
use std::{fs, path::Path};

pub fn read_wasm(path: impl AsRef<Path>, from_manifest_dir: bool) -> Vec<u8> {
    let path = if from_manifest_dir {
        fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../").join(path))
            .unwrap_or_else(|e| panic!("Failed to canonicalize path: {e}"))
    } else {
        PathBuf::from(path.as_ref())
    };

    println!("Reading WASM file at {}", path.display());

    fs::read(&path).unwrap_or_else(|e| panic!("Failed to read WASM file at {:?}: {e}", path))
}

pub static MT_RECEIVER_STUB_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(env!("DEFUSE_MULTI_TOKEN_RECEIVER_STUB_WASM"), false));

pub static DEFUSE_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm(env!("DEFUSE_WASM"), false));
pub static DEFUSE_LEGACY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("releases/previous.wasm", true));

pub static ESCROW_SWAP_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(env!("DEFUSE_ESCROW_SWAP_WASM"), false));

pub static POA_FACTORY_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm(env!("DEFUSE_POA_FACTORY_WASM"), false));

pub static NON_FUNGIBLE_TOKEN_WASM: LazyLock<Vec<u8>> =
    LazyLock::new(|| read_wasm("releases/non-fungible-token.wasm", true));

pub static WNEAR_WASM: LazyLock<Vec<u8>> = LazyLock::new(|| read_wasm("releases/wnear.wasm", true));
