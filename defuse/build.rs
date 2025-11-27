fn main() {
    // Check for unsupported configuration: native target without abi feature
    let is_wasm = std::env::var("TARGET")
        .map(|t| t.contains("wasm32"))
        .unwrap_or(false);
    let has_abi = std::env::var("CARGO_FEATURE_ABI").is_ok();

    if !is_wasm && !has_abi {
        println!("cargo::error=Native builds require the `abi` feature.");
        println!("cargo::error=Use one of:");
        println!("cargo::error=  - `cargo check --features abi` for native builds");
        println!("cargo::error=  - `cargo check --target wasm32-unknown-unknown` for wasm builds");
        println!("cargo::error=  - `cargo make build` for contract builds");
        std::process::exit(1);
    }
}
