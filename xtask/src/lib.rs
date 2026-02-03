mod builder;
mod contracts;

pub use builder::{
    BuildArtifact, BuildMode, BuildOptions, DEFUSE_BUILD_REPRODUCIBLE_ENV_VAR,
    DEFUSE_OUT_DIR_ENV_VAR,
};
pub use contracts::*;

use anyhow::Result;

use crate::builder::ContractBuilder;

pub fn build_contracts(
    contracts: Vec<ContractOptions>,
    options: BuildOptions,
) -> Result<Vec<BuildArtifact>> {
    ContractBuilder::new(contracts)
        .apply_options(options)
        .build_contracts()
}
