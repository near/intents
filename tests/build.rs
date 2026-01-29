// use cargo_near_build::{
//     BuildOpts, build_with_cli,
//     camino::Utf8PathBuf,
//     docker::{DockerBuildOpts, build as build_reproducible_with_cli},
// };
// use std::env;

// const DEFAULT_OUT_DIR: &str = "../res";
// const BUILD_REPRODUCIBLE_ENV_VAR: &str = "BUILD_REPRODUCIBLE";

// fn build_contract(
//     manifest: impl Into<Utf8PathBuf>,
//     features: &str,
//     outdir: impl Into<Utf8PathBuf>,
//     var: &str,
//     reproducible: bool,
// ) -> Result<(), Box<dyn std::error::Error + 'static>> {
//     let path = if reproducible {
//         let build_opts = DockerBuildOpts::builder()
//             .manifest_path(manifest.into())
//             .out_dir(outdir.into())
//             .build();

//         build_reproducible_with_cli(build_opts, false)?.path
//     } else {
//         let build_opts = BuildOpts::builder()
//             .manifest_path(manifest)
//             .features(features)
//             .out_dir(outdir)
//             .no_abi(true)
//             .build();

//         build_with_cli(build_opts)?
//     };

//     unsafe {
//         std::env::set_var(var, path);
//     }

//     Ok(())
// }

use xtask::BuildOptions;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    xtask::build_workspace_contracts(&BuildOptions::default())?;
    // // panic!("{}", env::var("PROFILE").unwrap());
    // let build_reproducible = env::var(BUILD_REPRODUCIBLE_ENV_VAR)
    //     .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str()));

    // let workdir = "..";
    // static CONTRACTS: &[(&str, &str, &str)] = &[
    //     #[cfg(feature = "defuse")]
    //     ("defuse", "contract,imt", "DEFUSE_WASM"),
    //     #[cfg(feature = "poa")]
    //     ("poa-token", "contract", "POA_TOKEN_WASM"),
    //     #[cfg(feature = "poa")]
    //     ("poa-factory", "contract", "POA_FACTORY_WASM"),
    //     #[cfg(feature = "escrow")]
    //     ("escrow-swap", "contract", "ESCROW_SWAP_WASM"),
    // ];

    // // 9af9f2b0460bd7dc9ae0bb77e4f8d4adb45824e3c8c5a58215f0e774672fa811

    // for (contract_dir, features, var) in CONTRACTS {
    //     let manifest = format!("{}/{}/Cargo.toml", workdir, contract_dir);

    //     build_contract(manifest, features, DEFAULT_OUT_DIR, var, build_reproducible)?;
    // }

    Ok(())
}
