mod builder;
mod contracts;

pub use builder::{
    BuildArtifact, BuildMode, BuildOptions, DEFUSE_BUILD_REPRODUCIBLE_ENV_VAR,
    DEFUSE_OUT_DIR_ENV_VAR,
};
pub use contracts::*;

use anyhow::Result;

use crate::builder::ContractBuilder;

#[macro_export]
macro_rules! cargo_warning {
    ($($arg:tt)*) => {
        println!("cargo::warning={}", format!($($arg)*));
    };
}

#[macro_export]
macro_rules! cargo_rerun_trigger {
    ($($arg:tt)*) => {
        println!("cargo::rerun-if-changed={}", format!($($arg)*));
    };
}

#[macro_export]
macro_rules! cargo_rerun_env_trigger {
    ($($arg:tt)*) => {
        println!("cargo::rerun-if-env-changed={}", format!($($arg)*));
    };
}

#[macro_export]
macro_rules! cargo_rustc_env {
    ($($arg:tt)*) => {
        println!("cargo::rustc-env={}", format!($($arg)*));
    };
}

pub fn build_contracts(
    contracts: Vec<ContractOptions>,
    options: BuildOptions,
) -> Result<Vec<BuildArtifact>> {
    ContractBuilder::new(contracts)
        .apply_options(options)
        .build_contracts()
}
