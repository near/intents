use clap::Args;

use anyhow::{Result, anyhow};
use cargo_near_build::{
    BuildOpts, build_with_cli,
    camino::Utf8PathBuf,
    docker::{DockerBuildOpts, build as build_reproducible_with_cli},
};
use std::env;

use crate::{Contract, ContractOptions};

const BUILD_REPRODUCIBLE_ENV_VAR: &str = "DEFUSE_BUILD_REPRODUCIBLE";
const DEFAULT_OUT_DIR: &str = "res";
const OUT_DIR_ENV_VAR: &str = "DEFUSE_OUT_DIR";

#[derive(Args, Clone, Default)]
pub struct BuildOptions {
    #[command(flatten)]
    reproducible: Option<ReproducibleBuildOptions>,
    #[arg(short, long)]
    outdir: Option<String>,
}

#[derive(Args, Clone, Default, Debug)]
pub struct ReproducibleBuildOptions {
    #[arg(short, long, default_value_t = true)]
    checksum: bool,
    #[arg(short, long, default_value_t = true)]
    parallel: bool,
}

#[derive(Debug, Clone)]
pub struct ContractBuilder {
    contracts: Vec<ContractOptions>,
    outdir: String,
    reproducible: Option<ReproducibleBuildOptions>,
}

impl ContractBuilder {
    pub fn new(contracts: Vec<ContractOptions>) -> Self {
        let reproducible = env::var(BUILD_REPRODUCIBLE_ENV_VAR)
            .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str()))
            .then_some(ReproducibleBuildOptions::default());
        let outdir = env::var(OUT_DIR_ENV_VAR).unwrap_or_else(|_| DEFAULT_OUT_DIR.to_string());

        Self {
            contracts,
            outdir,
            reproducible,
        }
    }

    pub fn apply_options(mut self, options: BuildOptions) -> Self {
        self = self.set_reproducible(options.reproducible);

        if let Some(outdir) = &options.outdir {
            self = self.set_outdir(outdir);
        }

        self
    }

    const fn set_reproducible(mut self, reproducible: Option<ReproducibleBuildOptions>) -> Self {
        self.reproducible = reproducible;
        self
    }

    fn set_outdir(mut self, outdir: impl Into<String>) -> Self {
        self.outdir = outdir.into();
        self
    }

    pub fn build_contracts(&self) -> Result<Vec<(Contract, Utf8PathBuf)>> {
        let workdir = env::var("CARGO_MANIFEST_DIR")?;
        let outdir = format!("{}/../{}/", workdir, self.outdir);

        if let Some(reproducible_opts) = &self.reproducible {
            if reproducible_opts.parallel {
                let handles = self
                    .contracts
                    .iter()
                    .map(|contract| {
                        let contract = contract.clone();
                        let outdir = outdir.clone();
                        let workdir = workdir.clone();
                        let checksum = reproducible_opts.checksum;

                        std::thread::spawn(move || {
                            Self::build_wasm_reproducible(
                                checksum,
                                &outdir,
                                &workdir,
                                contract.contract,
                            )
                        })
                    })
                    .collect::<Vec<_>>();

                handles
                    .into_iter()
                    .map(|h| {
                        h.join()
                            .map_err(|_| anyhow!("Thread panicked"))
                            .and_then(|r| r)
                    })
                    .collect::<Result<Vec<(Contract, Utf8PathBuf)>>>()
            } else {
                self.contracts
                    .iter()
                    .map(|contracts| Self::build_wasm(&outdir, &workdir, contracts))
                    .collect()
            }
        } else {
            self.contracts
                .iter()
                .map(|contracts| Self::build_wasm(&outdir, &workdir, contracts))
                .collect()
        }
    }

    fn build_wasm_reproducible(
        checksum: bool,
        outdir: &str,
        workdir: &str,
        contract: Contract,
    ) -> Result<(Contract, Utf8PathBuf)> {
        let spec = contract.spec();
        let manifest = format!("{}/../{}/Cargo.toml", workdir, spec.path);

        println!("Building contract: {} in reproducible mode", spec.name,);

        let build_opts = DockerBuildOpts::builder()
            .manifest_path(manifest.into())
            .out_dir(outdir.clone().into())
            .build();

        let artifacts = build_reproducible_with_cli(build_opts, false)
            .map_err(|e| anyhow!("Failed to build reproducible wasm: {e}"))?;

        if checksum {
            let checksum = artifacts
                .compute_hash()
                .map_err(|e| anyhow!("Failed to compute checksum: {e}"))?;

            println!("Computed checksum: {}", checksum.to_hex_string());

            std::fs::write(
                format!("{}/{}.sha256", outdir, spec.name),
                checksum.to_hex_string(),
            )?;
        }

        Ok((contract, artifacts.path))
    }

    fn build_wasm(
        outdir: &str,
        workdir: &str,
        ContractOptions { contract, features }: &ContractOptions,
    ) -> Result<(Contract, Utf8PathBuf)> {
        let spec = contract.spec();
        let manifest = format!("{}/../{}/Cargo.toml", workdir, spec.path);
        let features = features.clone().unwrap_or(spec.features.to_string());

        println!("Building contract: {} in non-reproducible mode", spec.name,);

        let build_opts = BuildOpts::builder()
            .manifest_path(manifest)
            .features(features)
            .out_dir(outdir)
            .no_abi(true)
            .build();

        let path = build_with_cli(build_opts).map_err(|e| anyhow!("Failed to build wasm: {e}"))?;

        Ok((contract.clone(), path))
    }
}
