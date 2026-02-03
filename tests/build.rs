#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(clippy))]
    {
        build::run()?;
    }

    Ok(())
}

#[cfg(not(clippy))]
mod build {

    use anyhow::{Result, anyhow};
    use std::path::Path;

    use cargo_metadata::{
        MetadataCommand, Package,
        camino::{Utf8Path, Utf8PathBuf},
    };

    const SKIP_CONTRACTS_BUILD_VAR: &str = "SKIP_CONTRACTS_BUILD";
    const TEST_OUTDIR: &str = "res";

    fn register_rebuild_triggers() -> Result<()> {
        println!("cargo:rerun-if-env-changed={SKIP_CONTRACTS_BUILD_VAR}");
        println!(
            "cargo:rerun-if-env-changed={}",
            xtask::DEFUSE_BUILD_REPRODUCIBLE_ENV_VAR
        );

        for member in get_workspace_members()? {
            println!("cargo:rerun-if-changed={member}");
        }

        println!("cargo:rerun-if-changed=../{TEST_OUTDIR}");

        Ok(())
    }

    fn is_excluded_package(pkg: &Package) -> bool {
        static EXCLUDED: &[&str] = &["sandbox", "tests"];

        EXCLUDED.iter().any(|p| pkg.name.contains(p))
    }

    fn get_workspace_members() -> Result<Vec<Utf8PathBuf>> {
        let root_manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");

        let metadata = MetadataCommand::new()
            .manifest_path(root_manifest_path)
            .no_deps()
            .exec()
            .map_err(|e| anyhow!("Failed to fetch cargo metadata: {e}"))?;

        let members = metadata
            .workspace_packages()
            .iter()
            .filter(|pkg| !is_excluded_package(pkg))
            .filter_map(|pkg| pkg.manifest_path.parent())
            .map(Utf8Path::to_path_buf)
            .collect();

        Ok(members)
    }

    pub fn run() -> Result<()> {
        register_rebuild_triggers()?;

        println!(
            "cargo:rustc-env={}={TEST_OUTDIR}",
            xtask::DEFUSE_OUT_DIR_ENV_VAR
        );

        let skip_build = std::env::var(SKIP_CONTRACTS_BUILD_VAR)
            .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str()));

        if skip_build {
            println!("Skipping contracts build due to {SKIP_CONTRACTS_BUILD_VAR} being set");
            return Ok(());
        }

        xtask::build_contracts(
            xtask::ContractOptions::all_without_features(),
            xtask::BuildOptions {
                outdir: Some(TEST_OUTDIR.to_string()),
                ..Default::default()
            },
        )?;

        Ok(())
    }
}
