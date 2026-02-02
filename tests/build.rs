use xtask::{BuildOptions, ContractOptions};

const DEFUSE_WASM_VAR: &str = "DEFUSE_WASM";
const POA_FACTORY_WASM_VAR: &str = "DEFUSE_POA_FACTORY_WASM";
const POA_TOKEN_WASM_VAR: &str = "DEFUSE_POA_TOKEN_WASM";
const ESCROW_SWAP_WASM_VAR: &str = "DEFUSE_ESCROW_SWAP_WASM";
const MULTI_TOKEN_RECEIVER_STUB_WASM_VAR: &str = "DEFUSE_MULTI_TOKEN_RECEIVER_STUB_WASM";

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let skip_build = std::env::var("SKIP_CONTRACTS_BUILD")
        .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str()));

    if skip_build {
        println!("Skipping contracts build due to SKIP_CONTRACTS_BUILD being set");
        return Ok(());
    }

    println!("cargo:rerun-if-changed=../defuse");
    println!("cargo:rerun-if-changed=../poa-factory");
    println!("cargo:rerun-if-changed=../poa-token");
    println!("cargo:rerun-if-changed=../escrow-swap");
    println!("cargo:rerun-if-changed=./contracts/multi-token-receiver-stub");

    let artifacts = xtask::build_contracts(
        ContractOptions::all_without_features(),
        BuildOptions::default(),
    )?;

    for a in artifacts {
        let env_var_key = match a.contract {
            xtask::Contract::Defuse => DEFUSE_WASM_VAR,
            xtask::Contract::PoaFactory => POA_FACTORY_WASM_VAR,
            xtask::Contract::PoaToken => POA_TOKEN_WASM_VAR,
            xtask::Contract::EscrowSwap => ESCROW_SWAP_WASM_VAR,
            xtask::Contract::MultiTokenReceiverStub => MULTI_TOKEN_RECEIVER_STUB_WASM_VAR,
        };

        println!("cargo:rustc-env={env_var_key}={}", a.wasm_path);
    }

    Ok(())
}
