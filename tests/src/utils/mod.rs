#![allow(dead_code)]

use std::fs;
use std::path::Path;

pub mod crypto;
pub mod fixtures;
pub mod payload;
pub mod test_log;

pub fn read_wasm(path: impl AsRef<Path>) -> Vec<u8> {
    let filename = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../")
        .join(path)
        .with_extension("wasm");

    fs::read(filename).unwrap()
}
