use std::{collections::BTreeMap, io};

use clap::Parser;
use defuse_global_deployer_core::State;
use near_account_id::AccountId;
use serde_with::{base64::Base64, ser::SerializeAsWrap};

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

    /// Pre-approve SHA-256 code hash: first `gd_deploy()` won't require `gd_approve()`.
    /// Hash can be encoded as base58 or hex with `0x` prefix
    #[arg(long, value_parser = parse_hash, value_name = "HASH")]
    pre_approve: Option<[u8; 32]>,

    /// Output single-line JSON with base64-encoded keys/values
    #[arg(short, long)]
    quiet: bool,
}

fn main() {
    let args = Args::parse();

    let state = State::owner(args.owner_id)
        .with_index(args.index)
        .pre_approve(args.pre_approve.unwrap_or(State::DEFAULT_HASH));

    if !args.quiet {
        eprintln!("// State:\n");
        serde_json::to_writer_pretty(io::stderr(), &state).expect("JSON");
        eprintln!("\n\n// Storage key-value pairs (as base64):\n");
    }

    let storage = state.as_storage();

    serde_json::to_writer(
        io::stdout(),
        #[allow(clippy::zero_sized_map_values)]
        &SerializeAsWrap::<_, BTreeMap<Base64, Base64>>::new(&storage),
    )
    .expect("JSON");
}

fn parse_hash(s: &str) -> Result<[u8; 32], String> {
    if let Some(s) = s.strip_prefix("0x") {
        hex::decode(s).map_err(|err| format!("hex: {err}"))?
    } else {
        bs58::decode(s)
            .into_vec()
            .map_err(|err| format!("base58: {err}"))?
    }
    .try_into()
    .map_err(|_| "hash must be 32 bytes encoded via hex or base58".to_string())
}
