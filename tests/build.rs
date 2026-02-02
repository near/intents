#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    #[cfg(not(clippy))]
    {
        const DEFUSE_BUILD_REPRODUCIBLE_VAR: &str = "DEFUSE_BUILD_REPRODUCIBLE";

        println!("cargo:rerun-if-changed=../defuse");
        println!("cargo:rerun-if-changed=../poa-factory");
        println!("cargo:rerun-if-changed=../poa-token");
        println!("cargo:rerun-if-changed=../escrow-swap");
        println!("cargo:rerun-if-changed=./contracts/multi-token-receiver-stub");

        println!("cargo:rerun-if-env-changed={DEFUSE_BUILD_REPRODUCIBLE_VAR}");

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
