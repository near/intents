mod builder;
mod contracts;

pub use builder::BuildOptions;
pub use contracts::Contract;

use anyhow::Result;

pub fn build_contract(contract: Contract, options: &BuildOptions) -> Result<()> {
    builder::build_contract(contract, options)
}

pub fn build_workspace_contracts(options: &BuildOptions) -> Result<()> {
    builder::build_workspace_contracts(options)
}
