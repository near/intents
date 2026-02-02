use xtask::{BuildOptions, Contract, ContractOptions};

const POA_TOKEN_WASM_VAR: &str = "POA_TOKEN_WASM";

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    println!("cargo:rerun-if-changed=../poa-token");

    let res = xtask::build_contracts(
        vec![ContractOptions::new_without_features(Contract::PoaToken)],
        BuildOptions::default(),
    )?;

    let artifact = res.first().ok_or("No contract artifacts were built")?;

    println!(
        "cargo:rustc-env={POA_TOKEN_WASM_VAR}={}",
        artifact.wasm_path
    );

    Ok(())
}
