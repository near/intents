use std::{collections::BTreeMap, io};

use anyhow::Context;
use clap::Parser;
use defuse_cli_utils::hash::HashSource;
use defuse_global_deployer_core::State;
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
