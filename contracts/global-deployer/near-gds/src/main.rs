use std::{collections::BTreeMap, io};

use anyhow::Context;
use clap::Parser;
use defuse_cli_utils::hash::HashSource;
use defuse_global_deployer::State;
use near_account_id::AccountId;
use serde_with::{base64::Base64, ser::SerializeAsWrap};
use sha2::Sha256;

#[derive(Parser)]
/// Print JSON storage key-value pairs (as base64) for `StateInit`
/// of a Global Deployer contract
struct Args {
    /// Owner account ID
    #[arg(long, value_name = "AccountId")]
    owner_id: AccountId,

    /// Unique index for the deployer instance.
    /// Can be used to derive multiple deployers for a single owner
    #[arg(long, short, default_value_t = 0, value_name = "N")]
    index: u32,

    /// Pre-approve SHA-256 code hash, so that first `gd_deploy()` won't
    /// require `gd_approve()` before it.
    ///
    /// `HASH` can be encoded as base58 or hex with `0x` prefix.
    /// `@FILE` will calculate SHA-256 hash of the `FILE` contents.
    /// `@-` will calculate SHA-256 hash of the stdin contents.
    #[arg(long, value_name = "HASH | @FILE | @-")]
    pre_approve: Option<HashSource<Sha256>>,

    /// Output single-line JSON with base64-encoded keys/values
    #[arg(short, long)]
    quiet: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let pre_approve = if let Some(pre_approve) = args.pre_approve {
        pre_approve.hash().context("pre_approve")?
    } else {
        State::DEFAULT_HASH
    };

    let state = State::owner(args.owner_id)
        .with_index(args.index)
        .pre_approve(pre_approve);

    if !args.quiet {
        eprintln!("// State:");
        serde_json::to_writer_pretty(io::stderr(), &state).context("JSON")?;
        eprintln!("\n\n// Storage key-value pairs (as base64):");
    }

    let storage = state.as_storage();

    serde_json::to_writer(
        io::stdout(),
        #[allow(clippy::zero_sized_map_values)]
        &SerializeAsWrap::<_, BTreeMap<Base64, Base64>>::new(&storage),
    )
    .context("JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    fn readme_text() -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../README.md");
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
    }

    fn usage_section(readme: &str) -> &str {
        let start = readme
            .find("### Usage")
            .unwrap_or_else(|| panic!("README is missing a '### Usage' section"));
        let rest = &readme[start..];
        let end = rest.find("### Example").unwrap_or(rest.len());
        &rest[..end]
    }

    #[test]
    fn readme_install_path_exists() {
        let readme = readme_text();
        let paths: Vec<&str> = readme
            .lines()
            .filter_map(|line| line.trim().strip_prefix("cargo install --path "))
            .collect();
        assert!(
            !paths.is_empty(),
            "README should document `cargo install --path`"
        );
        for rel in paths {
            let resolved = workspace_root().join(rel.trim_start_matches("./"));
            assert!(
                Path::new(&resolved).exists(),
                "documented install path `{rel}` does not exist (resolved to {})",
                resolved.display()
            );
        }
    }

    #[test]
    fn readme_usage_matches_cli_flags() {
        let readme = readme_text();
        let section = usage_section(&readme);
        let help = crate::Args::command().render_long_help().to_string();

        assert!(
            help.contains("--pre-approve"),
            "CLI --help must include --pre-approve:\n{help}"
        );
        assert!(
            section.contains("--pre-approve"),
            "README Usage/help must document --pre-approve:\n{section}"
        );
        assert!(
            !section.contains("--approve <"),
            "README Usage/help still documents the stale --approve flag; the CLI flag is --pre-approve:\n{section}"
        );
        assert!(
            section.contains("@FILE"),
            "README Usage/help should document @FILE / @- hash sources, as --help does:\n{section}"
        );
    }
}
