use clap::{Args, Subcommand};

use anyhow::{Result, anyhow};
use cargo_near_build::{
    BuildArtifact as CompilationArtifact, BuildOpts, build as build_non_reproducible_wasm,
    camino::Utf8PathBuf,
    docker::{DockerBuildOpts, build as build_reproducible_wasm},
};
use std::env;

use crate::{Contract, cargo_warning};

pub const DEFUSE_BUILD_REPRODUCIBLE_ENV_VAR: &str = "DEFUSE_BUILD_REPRODUCIBLE";
pub const DEFUSE_OUT_DIR_ENV_VAR: &str = "DEFUSE_OUT_DIR";
pub const DEFAULT_OUT_DIR: &str = "res";

#[derive(Subcommand, Debug, Clone)]
pub enum BuildMode {
    NonReproducible(NonReproducibleBuildOptions),
    Reproducible(ReproducibleBuildOptions),
}

impl Default for BuildMode {
    fn default() -> Self {
        Self::NonReproducible(NonReproducibleBuildOptions::default())
    }
}

#[derive(Args, Clone, Default)]
pub struct BuildOptions {
    #[command(subcommand)]
    pub mode: BuildMode,

    #[arg(short, long)]
    pub outdir: Option<String>,
}

#[derive(Args, Clone, Default, Debug)]
pub struct ReproducibleBuildOptions {
    #[arg(short, long, default_value_t = true)]
    pub checksum: bool,

    #[arg(short, long)]
    pub variant: Option<String>,
}

#[derive(Args, Clone, Default, Debug)]
pub struct NonReproducibleBuildOptions {
    #[arg(short, long)]
    pub features: Option<String>,
}

#[derive(Debug, Clone)]
struct BuildContext {
    repo_root: Utf8PathBuf,
    outdir: Utf8PathBuf,
}

#[derive(Debug, Clone)]
pub struct BuildArtifact {
    pub contract: Contract,
    pub wasm_path: Utf8PathBuf,
    pub checksum_hex: Option<String>,
    pub checksum_path: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ContractBuilder {
    contract: Contract,
    outdir: String,
    mode: BuildMode,
}

impl ContractBuilder {
    pub fn new(contract: Contract) -> Self {
        let reproducible = env::var(DEFUSE_BUILD_REPRODUCIBLE_ENV_VAR)
            .is_ok_and(|v| !["0", "false"].contains(&v.to_lowercase().as_str()));
        let mode = if reproducible {
            BuildMode::Reproducible(ReproducibleBuildOptions::default())
        } else {
            BuildMode::NonReproducible(NonReproducibleBuildOptions::default())
        };

        let outdir =
            env::var(DEFUSE_OUT_DIR_ENV_VAR).unwrap_or_else(|_| DEFAULT_OUT_DIR.to_string());

        Self {
            contract,
            outdir,
            mode,
        }
    }

    pub fn apply_options(mut self, options: BuildOptions) -> Self {
        self = self.set_mode(options.mode);

        if let Some(outdir) = &options.outdir {
            self = self.set_outdir(outdir);
        }

        self
    }

    pub fn set_mode(mut self, mode: BuildMode) -> Self {
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

    pub fn build_contract(self) -> Result<BuildArtifact> {
        let ctx = self.build_context()?;

        match &self.mode {
            BuildMode::NonReproducible(opts) => {
                Self::build_non_reproducible(&ctx, self.contract, opts)
            }
            BuildMode::Reproducible(opts) => Self::build_reproducible(&ctx, self.contract, opts),
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
            .map_err(|e| anyhow!("failed to compute checksum: {e}"))?;

        let checksum_hex = checksum.to_hex_string();
        cargo_warning!("xtask build: computed checksum: {checksum_hex}",);

        let checksum_path = ctx.outdir.join(format!("{name}.sha256"));
        std::fs::write(checksum_path.as_str(), &checksum_hex)?;

        Ok((Some(checksum_hex), Some(checksum_path)))
    }

    fn build_reproducible(
        ctx: &BuildContext,
        contract: Contract,
        options: &ReproducibleBuildOptions,
    ) -> Result<BuildArtifact> {
        let spec = contract.spec();
        let manifest = ctx.repo_root.join(spec.path).join("Cargo.toml");

        cargo_warning!(
            "xtask build: reproducible {} variant={} outdir={}",
            spec.name,
            options.variant.as_deref().unwrap_or("default"),
            ctx.outdir
        );

        let build_opts = DockerBuildOpts::builder()
            .manifest_path(manifest)
            .out_dir(ctx.outdir.clone())
            .maybe_variant(options.variant.clone())
            .build();

        let artifacts = build_reproducible_wasm(build_opts, false)
            .map_err(|e| anyhow!("failed to build reproducible wasm: {e}"))?;

        let (checksum_hex, checksum_path) =
            Self::maybe_compute_checksum(&artifacts, ctx, spec.name, options.checksum)?;

        Ok(BuildArtifact {
            contract,
            wasm_path: artifacts.path,
            checksum_hex,
            checksum_path,
        })
    }

    fn build_non_reproducible(
        ctx: &BuildContext,
        contract: Contract,
        options: &NonReproducibleBuildOptions,
    ) -> Result<BuildArtifact> {
        let spec = contract.spec();
        let manifest = ctx.repo_root.join(spec.path).join("Cargo.toml");
        let features = options
            .features
            .clone()
            .unwrap_or_else(|| spec.features.to_string());

        cargo_warning!(
            "xtask build: non-reproducible {} features={features} outdir={}",
            spec.name,
            ctx.outdir
        );

        let build_opts = BuildOpts::builder()
            .manifest_path(manifest)
            .features(features)
            .out_dir(ctx.outdir.as_str())
            .no_abi(true)
            .build();

        let artifact = build_non_reproducible_wasm(build_opts)
            .map_err(|e| anyhow!("failed to build wasm: {e}"))?;

        Ok(BuildArtifact {
            contract,
            wasm_path: artifact.path,
            checksum_hex: None,
            checksum_path: None,
        })
    }
}
