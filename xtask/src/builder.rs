use clap::Args;

use anyhow::{Result, anyhow};
use cargo_near_build::{
    BuildOpts, build_with_cli,
    docker::{DockerBuildOpts, build as build_reproducible_with_cli},
};
use std::env;

use crate::Contract;

const DEFAULT_OUT_DIR: &str = "res";
const BUILD_REPRODUCIBLE_ENV_VAR: &str = "DEFUSE_BUILD_REPRODUCIBLE";

#[derive(Args, Clone, Default)]
pub struct BuildOptions {
    #[arg(short, long, default_value_t = true)]
    reproducible: bool,
    #[arg(short, long, default_value_t = false)]
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
    env_var_key: String,
    path: String,
    reproducible: bool,
    checksum: bool,
    outdir: String,
}

impl ContractBuilder {
    fn new(contract: Contract) -> Self {
        let spec = contract.spec();

        let reproducible = env::var(BUILD_REPRODUCIBLE_ENV_VAR)
            .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str()));

        Self {
            name: spec.name.to_string(),
            features: spec.features.to_string(),
            env_var_key: spec.env_var_key.to_string(),
            path: spec.path.to_string(),
            outdir: DEFAULT_OUT_DIR.to_string(),
            reproducible: reproducible,
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
        if let Some(env_var_key) = &options.env_var {
            self = self.env_var_key(env_var_key);
        }
        if let Some(outdir) = &options.outdir {
            self = self.set_outdir(outdir);
        }

        self
    }

    fn set_reproducible(mut self, reproducible: bool) -> Self {
        self.reproducible = reproducible;
        self
    }

    fn set_checksum(mut self, checksum: bool) -> Self {
        self.checksum = checksum;
        self
    }

    fn set_features(mut self, features: impl Into<String>) -> Self {
        self.features = features.into();
        self
    }

    fn env_var_key(mut self, env_var_key: impl Into<String>) -> Self {
        self.env_var_key = env_var_key.into();
        self
    }

    fn set_outdir(mut self, outdir: impl Into<String>) -> Self {
        self.outdir = outdir.into();
        self
    }

    fn post_build_setup(mut self) -> Self {
        self
    }

    fn build_contract(self) -> Result<()> {
        let workdir = env::var("CARGO_MANIFEST_DIR")?;
        let manifest = format!("{}/../{}/Cargo.toml", workdir, self.path);
        let outdir = format!("{}/../{}/", workdir, self.outdir);

        let path = if self.reproducible {
            let build_opts = DockerBuildOpts::builder()
                .manifest_path(manifest.into())
                .out_dir(outdir.into())
                .build();

            build_reproducible_with_cli(build_opts, false)
                .map_err(|e| anyhow!("Failed to build reproducible wasm: {}", e))?
                .path
        } else {
            let build_opts = BuildOpts::builder()
                .manifest_path(manifest)
                .features(self.features)
                .out_dir(outdir)
                .no_abi(true)
                .build();

            build_with_cli(build_opts).map_err(|e| anyhow!("Failed to build wasm: {}", e))?
        };

        // unsafe {
        //     std::env::set_var(var, path);
        // }

        Ok(())
    }
}

pub fn build_contract(contract: Contract, options: &BuildOptions) -> Result<()> {
    ContractBuilder::new(contract)
        .apply_options(&options)
        .build_contract()
}

pub fn build_workspace_contracts(options: &BuildOptions) -> Result<()> {
    let contracts = [
        Contract::Defuse,
        Contract::PoaFactory,
        Contract::PoaToken,
        Contract::EscrowSwap,
        Contract::MultiTokenReceiverStub,
    ];

    contracts
        .into_iter()
        .map(|contract| build_contract(contract, options))
        .collect::<Result<()>>()
}
