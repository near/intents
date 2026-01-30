mod builder;
mod contracts;

pub use builder::BuildOptions;
use cargo_near_build::camino::Utf8PathBuf;
pub use contracts::*;

use anyhow::Result;

use crate::builder::ContractBuilder;

pub fn build_contracts(
    contracts: Vec<ContractOptions>,
    options: BuildOptions,
) -> Result<Vec<(Contract, Utf8PathBuf)>> {
    ContractBuilder::new(contracts)
        .apply_options(options)
        .build_contracts()
}
