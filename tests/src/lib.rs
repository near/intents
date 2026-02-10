#![allow(clippy::too_many_lines)]

pub mod wasms;

pub use defuse_sandbox as sandbox;
pub use defuse_test_utils as utils;

#[cfg(test)]
mod tests;
