#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // #[cfg(not(clippy))]
    // {
    //     build::run()?;
    // }

    Ok(())
}

// #[cfg(not(clippy))]
// mod build {

//     use anyhow::{Result, anyhow};
//     use std::{env, path::Path};

//     use cargo_metadata::{
//         MetadataCommand, Package,
//         camino::{Utf8Path, Utf8PathBuf},
//     };

//     use xtask::{
//         Contract, DEFUSE_BUILD_REPRODUCIBLE_ENV_VAR, DEFUSE_OUT_DIR_ENV_VAR,
//         cargo_rerun_env_trigger, cargo_rerun_trigger, cargo_warning,
//     };

//     const SKIP_CONTRACTS_BUILD_VAR: &str = "DEFUSE_SKIP_CONTRACTS_BUILD";
//     const CARGO_OUT_DIR_ENV_VAR: &str = "OUT_DIR";

//     fn register_rebuild_triggers(out_dir: &str) -> Result<()> {
//         cargo_rerun_env_trigger!("{SKIP_CONTRACTS_BUILD_VAR}");
//         cargo_rerun_env_trigger!("{DEFUSE_BUILD_REPRODUCIBLE_ENV_VAR}");

//         // for member in get_workspace_members()? {
//         //     cargo_rerun_trigger!("{member}");
//         // }

//         // cargo_rerun_trigger!("{out_dir}");
//         // cargo_rerun_trigger!("../Cargo.lock");

//         Ok(())
//     }

//     fn is_excluded_package(pkg: &Package) -> bool {
//         static EXCLUDED: &[&str] = &["sandbox", "tests"];

//         EXCLUDED.iter().any(|p| pkg.name.contains(p))
//     }

//     fn get_workspace_members() -> Result<Vec<Utf8PathBuf>> {
//         let root_manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");

//         let metadata = MetadataCommand::new()
//             .manifest_path(root_manifest_path)
//             .no_deps()
//             .exec()
//             .map_err(|e| anyhow!("failed to fetch cargo metadata: {e}"))?;

//         let members = metadata
//             .workspace_packages()
//             .iter()
//             .filter(|pkg| !is_excluded_package(pkg))
//             .filter_map(|pkg| pkg.manifest_path.parent())
//             .map(Utf8Path::to_path_buf)
//             .collect();

//         Ok(members)
//     }

//     pub fn run() -> Result<()> {
//         // Use cargo OUT_DIR in case if custom DEFUSE_OUT_DIR is not set
//         // at least one of these variables is set during script execution
//         let out_dir = env::var(DEFUSE_OUT_DIR_ENV_VAR)
//             .unwrap_or_else(|_| env::var(CARGO_OUT_DIR_ENV_VAR).unwrap());

//         register_rebuild_triggers(&out_dir)?;

//         let skip_build = std::env::var(SKIP_CONTRACTS_BUILD_VAR)
//             .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str()));

//         if skip_build {
//             cargo_warning!("Skipping contracts build due to {SKIP_CONTRACTS_BUILD_VAR} being set");
//             return Ok(());
//         }

//         cargo_warning!("Building contracts into: {out_dir}");

//         let opts = xtask::BuildOptions {
//             outdir: Some(out_dir),
//             ..Default::default()
//         };
//         let contracts = Contract::all()
//             .into_iter()
//             .map(|c| (c, opts.clone()))
//             .collect();

//         xtask::build_contracts(contracts)?;

//         Ok(())
//     }
// }
