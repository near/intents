mod builder;
mod contracts;

pub use builder::{BuildArtifact, BuildMode, BuildOptions};
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
