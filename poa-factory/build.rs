// use xtask::{BuildOptions, Contract, ContractOptions};

// const POA_TOKEN_WASM_VAR: &str = "POA_TOKEN_WASM";

// fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
//     println!("cargo:rerun-if-changed=../poa-token");

//     let res = xtask::build_contracts(
//         vec![ContractOptions::new_without_features(Contract::PoaToken)],
//         BuildOptions::default(),
//     )?;

//     let artifact = res.first().ok_or("No contract artifacts were built")?;

//     println!(
//         "cargo:rustc-env={POA_TOKEN_WASM_VAR}={}",
//         artifact.wasm_path
//     );

//     Ok(())
// }

use cargo_near_build::{
    BuildOpts, bon,
    camino::Utf8PathBuf,
    extended::{BuildOptsExtended, build_with_cli},
};
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let workdir = "../poa-token";
    let env_var_key = "POA_TOKEN_WASM";
    let manifest = Utf8PathBuf::from_str(workdir)?.join("Cargo.toml");

    let build_opts = BuildOpts::builder()
        .manifest_path(manifest)
        .features("contract")
        .no_abi(true)
        .build();

    let extended_build_opts = BuildOptsExtended::builder()
        .build_opts(build_opts)
        .rerun_if_changed_list(bon::vec!["../Cargo.toml", "../Cargo.lock"])
        .result_file_path_env_key(env_var_key)
        .prepare()?;

    build_with_cli(extended_build_opts)?;

    Ok(())
}
