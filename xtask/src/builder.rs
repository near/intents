use clap::Args;

use anyhow::{Result, anyhow};
use cargo_near_build::{
    BuildArtifact as CompilationArtifact, BuildOpts, build_with_cli,
    camino::Utf8PathBuf,
    docker::{DockerBuildOpts, build as build_reproducible_with_cli},
};
use std::env;

use crate::{Contract, ContractOptions};

pub const DEFUSE_BUILD_REPRODUCIBLE_ENV_VAR: &str = "DEFUSE_BUILD_REPRODUCIBLE";
pub const DEFUSE_OUT_DIR_ENV_VAR: &str = "DEFUSE_OUT_DIR";
pub const DEFAULT_OUT_DIR: &str = "res";

#[derive(Debug, Clone)]
pub enum BuildMode {
    NonReproducible,
    Reproducible(ReproducibleBuildOptions),
}

#[derive(Debug, Clone)]
pub struct BuildArtifact {
    pub contract: Contract,
    pub wasm_path: Utf8PathBuf,
    pub checksum_hex: Option<String>,
    pub checksum_path: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone)]
struct BuildContext {
    repo_root: Utf8PathBuf,
    outdir: Utf8PathBuf,
}

#[derive(Args, Clone, Default)]
pub struct BuildOptions {
    #[arg(short, long)]
    pub reproducible: bool,
    #[command(flatten)]
    pub reproducible_options: Option<ReproducibleBuildOptions>,
    #[arg(short, long)]
    pub outdir: Option<String>,
}

#[derive(Args, Clone, Default, Debug)]
pub struct ReproducibleBuildOptions {
    #[arg(short, long, default_value_t = true)]
    pub checksum: bool,
}

#[derive(Debug, Clone)]
pub struct ContractBuilder {
    contracts: Vec<ContractOptions>,
    outdir: String,
    mode: BuildMode,
}

impl ContractBuilder {
    pub fn new(contracts: Vec<ContractOptions>) -> Self {
        let reproducible = env::var(DEFUSE_BUILD_REPRODUCIBLE_ENV_VAR)
            .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str()));
        let mode = if reproducible {
            BuildMode::Reproducible(ReproducibleBuildOptions::default())
        } else {
            BuildMode::NonReproducible
        };
        let outdir =
            env::var(DEFUSE_OUT_DIR_ENV_VAR).unwrap_or_else(|_| DEFAULT_OUT_DIR.to_string());

        Self {
            contracts,
            outdir,
            mode,
        }
    }

    pub fn apply_options(mut self, options: BuildOptions) -> Self {
        if options.reproducible {
            let options = options
                .reproducible_options
                .map_or_else(ReproducibleBuildOptions::default, |opts| opts);

            self = self.set_mode(BuildMode::Reproducible(options));
        }

        if let Some(outdir) = &options.outdir {
            self = self.set_outdir(outdir);
        }

        self
    }

    pub const fn set_mode(mut self, mode: BuildMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn set_outdir(mut self, outdir: impl Into<String>) -> Self {
        self.outdir = outdir.into();
        self
    }

    fn build_context(&self) -> Result<BuildContext> {
        let workdir = Utf8PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
        let repo_root = workdir.join("..");
        let outdir = repo_root.join(&self.outdir);

        Ok(BuildContext { repo_root, outdir })
    }

    pub fn build_contracts(&self) -> Result<Vec<BuildArtifact>> {
        let ctx = self.build_context()?;

        self.contracts
            .iter()
            .map(|c| self.build_one(&ctx, c))
            .collect()
    }

    fn build_one(&self, ctx: &BuildContext, contract: &ContractOptions) -> Result<BuildArtifact> {
        match &self.mode {
            BuildMode::NonReproducible => Self::build_non_reproducible(ctx, contract),
            BuildMode::Reproducible(opts) => {
                Self::build_reproducible(ctx, &contract.contract, opts.checksum)
            }
        }
    }

    fn maybe_compute_checksum(
        artifacts: &CompilationArtifact,
        ctx: &BuildContext,
        name: &str,
        enabled: bool,
    ) -> Result<(Option<String>, Option<Utf8PathBuf>)> {
        if !enabled {
            return Ok((None, None));
        }

        let checksum = artifacts
            .compute_hash()
            .map_err(|e| anyhow!("Failed to compute checksum: {e}"))?;

        let checksum_hex = checksum.to_hex_string();
        println!("Computed checksum: {checksum_hex}",);

        let checksum_path = ctx.outdir.join(format!("{name}.sha256"));
        std::fs::write(checksum_path.as_str(), &checksum_hex)?;

        Ok((Some(checksum_hex), Some(checksum_path)))
    }

    fn build_reproducible(
        ctx: &BuildContext,
        contract: &Contract,
        checksum: bool,
    ) -> Result<BuildArtifact> {
        let spec = contract.spec();
        let manifest = ctx.repo_root.join(spec.path).join("Cargo.toml");

        println!("Building contract: {} in reproducible mode", spec.name,);

        let build_opts = DockerBuildOpts::builder()
            .manifest_path(manifest)
            .out_dir(ctx.outdir.clone())
            .build();

        let artifacts = build_reproducible_with_cli(build_opts, false)
            .map_err(|e| anyhow!("Failed to build reproducible wasm: {e}"))?;

        let (checksum_hex, checksum_path) =
            Self::maybe_compute_checksum(&artifacts, ctx, spec.name, checksum)?;

        Ok(BuildArtifact {
            contract: contract.clone(),
            wasm_path: artifacts.path,
            checksum_hex,
            checksum_path,
        })
    }

    fn build_non_reproducible(
        ctx: &BuildContext,
        ContractOptions { contract, features }: &ContractOptions,
    ) -> Result<BuildArtifact> {
        let spec = contract.spec();
        let manifest = ctx.repo_root.join(spec.path).join("Cargo.toml");
        let features = features
            .clone()
            .unwrap_or_else(|| spec.features.to_string());

        println!("Building contract: {} in non-reproducible mode", spec.name,);

        let build_opts = BuildOpts::builder()
            .manifest_path(manifest)
            .features(features)
            .out_dir(ctx.outdir.as_str())
            .no_abi(true)
            .build();

        let wasm_path =
            build_with_cli(build_opts).map_err(|e| anyhow!("Failed to build wasm: {e}"))?;

        Ok(BuildArtifact {
            contract: contract.clone(),
            wasm_path,
            checksum_hex: None,
            checksum_path: None,
        })
    }
}
