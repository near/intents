use cargo_near_build::{
    BuildOpts, bon,
    camino::Utf8PathBuf,
    env_keys,
    extended::{BuildOptsExtended, BuildScriptOpts, build},
};
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let workdir = "../poa-token";
    let nep330_contract_path = "./poa-token";
    let cargo_override = "../target/build-poa-token";
    let stub_path = "../target/poa-token.wasm";
    let env_var_key = "POA_TOKEN_WASM";

    let manifest = Utf8PathBuf::from_str(&workdir)?.join("Cargo.toml");

    let build_opts = BuildOpts::builder()
        .manifest_path(manifest)
        .override_nep330_contract_path(nep330_contract_path)
        .override_cargo_target_dir(cargo_override)
        .no_abi(true)
        .build();

    let build_script_opts = BuildScriptOpts::builder()
        .rerun_if_changed_list(bon::vec![workdir, "Cargo.toml", "../Cargo.lock"])
        .build_skipped_when_env_is(vec![(env_keys::BUILD_RS_ABI_STEP_HINT, "true")])
        .stub_path(stub_path)
        .result_env_key(env_var_key)
        .build();

    let extended_opts = BuildOptsExtended::builder()
        .build_opts(build_opts)
        .build_script_opts(build_script_opts)
        .build();

    build(extended_opts)?;

    Ok(())
}
