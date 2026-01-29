mod builder;
mod contracts;

pub use builder::BuildOptions;
use cargo_near_build::camino::Utf8PathBuf;
pub use contracts::*;

use anyhow::Result;

pub fn build_contract(contract: &Contract, options: &BuildOptions) -> Result<Utf8PathBuf> {
    builder::build_contract(contract, options)
}

pub fn build_workspace_contracts(options: &BuildOptions) -> Result<Vec<(Contract, Utf8PathBuf)>> {
    builder::build_workspace_contracts(options)
}
