use clap::Args;

use anyhow::{Result, anyhow};
use cargo_near_build::{
    BuildOpts, build_with_cli,
    camino::Utf8PathBuf,
    docker::{DockerBuildOpts, build as build_reproducible_with_cli},
};
use std::env;

use crate::Contract;

const BUILD_REPRODUCIBLE_ENV_VAR: &str = "DEFUSE_BUILD_REPRODUCIBLE";
const DEFAULT_OUT_DIR: &str = "res";
const OUT_DIR_ENV_VAR: &str = "DEFUSE_OUT_DIR";

#[derive(Args, Clone, Default)]
pub struct BuildOptions {
    #[arg(short, long)]
    reproducible: bool,
    #[arg(short, long)]
    checksum: bool,
    #[arg(short, long)]
    features: Option<String>,
    #[arg(short, long)]
    env_var: Option<String>,
    #[arg(short, long)]
    outdir: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContractBuilder {
    name: String,
    features: String,
    path: String,
    reproducible: bool,
    checksum: bool,
    outdir: String,
}

impl ContractBuilder {
    fn new(contract: &Contract) -> Self {
        let spec = contract.spec();

        let reproducible = env::var(BUILD_REPRODUCIBLE_ENV_VAR)
            .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str()));
        let outdir = env::var(OUT_DIR_ENV_VAR).unwrap_or_else(|_| DEFAULT_OUT_DIR.to_string());

        Self {
            name: spec.name.to_string(),
            features: spec.features.to_string(),
            path: spec.path.to_string(),
            outdir,
            reproducible,
            checksum: false,
        }
    }

    fn apply_options(mut self, options: &BuildOptions) -> Self {
        self = self
            .set_reproducible(options.reproducible)
            .set_checksum(options.checksum);

        if let Some(features) = &options.features {
            self = self.set_features(features);
        }

        if let Some(outdir) = &options.outdir {
            self = self.set_outdir(outdir);
        }

        self
    }

    const fn set_reproducible(mut self, reproducible: bool) -> Self {
        self.reproducible = reproducible;
        self
    }

    const fn set_checksum(mut self, checksum: bool) -> Self {
        self.checksum = checksum;
        self
    }

    fn set_features(mut self, features: impl Into<String>) -> Self {
        self.features = features.into();
        self
    }

    fn set_outdir(mut self, outdir: impl Into<String>) -> Self {
        self.outdir = outdir.into();
        self
    }

    fn post_build_setup(mut self) -> Self {
        self
    }

    fn build_contract(self) -> Result<Utf8PathBuf> {
        let workdir = env::var("CARGO_MANIFEST_DIR")?;
        let manifest = format!("{}/../{}/Cargo.toml", workdir, self.path);
        let outdir = format!("{}/../{}/", workdir, self.outdir);

        println!(
            "Building contract: {} in {} mode",
            self.name,
            if self.reproducible {
                "reproducible"
            } else {
                "non-reproducible"
            }
        );

        let path = if self.reproducible {
            let build_opts = DockerBuildOpts::builder()
                .manifest_path(manifest.into())
                .out_dir(outdir.into())
                .build();

            build_reproducible_with_cli(build_opts, false)
                .map_err(|e| anyhow!("Failed to build reproducible wasm: {e}"))?
                .path
        } else {
            let build_opts = BuildOpts::builder()
                .manifest_path(manifest)
                .features(self.features)
                .out_dir(outdir)
                .no_abi(true)
                .build();

            build_with_cli(build_opts).map_err(|e| anyhow!("Failed to build wasm: {e}"))?
        };

        // println!("cargo::rerun-if-changed=src/hello.c");

        // // For build scripts
        // println!("cargo:rustc-env={}={}", self.env_var_key, path);
        // // For regular run
        // unsafe {
        //     std::env::set_var(self.env_var_key, path);
        // }

        Ok(path)
    }
}

pub fn build_contract(contract: &Contract, options: &BuildOptions) -> Result<Utf8PathBuf> {
    ContractBuilder::new(contract)
        .apply_options(options)
        .build_contract()
}

pub fn build_workspace_contracts(options: &BuildOptions) -> Result<Vec<(Contract, Utf8PathBuf)>> {
    Contract::all()
        .into_iter()
        .map(|contract| build_contract(&contract, options).map(|path| (contract, path)))
        .collect::<Result<Vec<(Contract, Utf8PathBuf)>>>()
}
