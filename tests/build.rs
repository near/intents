#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    #[cfg(not(clippy))]
    {
        const SKIP_CONTRACTS_BUILD_VAR: &str = "SKIP_CONTRACTS_BUILD";
        const TEST_OUTDIR: &str = "res";

        println!("cargo:rerun-if-env-changed={SKIP_CONTRACTS_BUILD_VAR}");
        println!(
            "cargo:rerun-if-env-changed={}",
            xtask::DEFUSE_BUILD_REPRODUCIBLE_ENV_VAR
        );

        println!("cargo:rerun-if-changed=../defuse");
        println!("cargo:rerun-if-changed=../poa-factory");
        println!("cargo:rerun-if-changed=../poa-token");
        println!("cargo:rerun-if-changed=../escrow-swap");
        println!("cargo:rerun-if-changed=./contracts/multi-token-receiver-stub");

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
    }

    Ok(())
}
