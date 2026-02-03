#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    #[cfg(not(clippy))]
    {
        const DEFUSE_BUILD_REPRODUCIBLE_VAR: &str = "DEFUSE_BUILD_REPRODUCIBLE";
        const SKIP_CONTRACTS_BUILD_VAR: &str = "SKIP_CONTRACTS_BUILD";

        println!("cargo:rerun-if-env-changed={SKIP_CONTRACTS_BUILD_VAR}");
        println!("cargo:rerun-if-env-changed={DEFUSE_BUILD_REPRODUCIBLE_VAR}");

        println!("cargo:rerun-if-changed=../defuse");
        println!("cargo:rerun-if-changed=../poa-factory");
        println!("cargo:rerun-if-changed=../poa-token");
        println!("cargo:rerun-if-changed=../escrow-swap");
        println!("cargo:rerun-if-changed=./contracts/multi-token-receiver-stub");

        let skip_build = std::env::var(SKIP_CONTRACTS_BUILD_VAR)
            .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str()));

        if skip_build {
            println!("Skipping contracts build due to {SKIP_CONTRACTS_BUILD_VAR} being set");
            return Ok(());
        }

        let artifacts = xtask::build_contracts(
            xtask::ContractOptions::all_without_features(),
            xtask::BuildOptions::default(),
        )?;

        for a in artifacts {
            println!(
                "cargo:rustc-env={}={}",
                a.contract.default_env(),
                a.wasm_path
            );
        }
    }

    Ok(())
}
