use std::{collections::BTreeMap, io};

use anyhow::Context;
use clap::Parser;
use defuse_cli_utils::hash::HashSource;
use defuse_outlayer_app_core::{AdminPublicKey, State};
use near_account_id::AccountId;
use serde_with::{base64::Base64, ser::SerializeAsWrap};
use sha2::Sha256;
use url::Url;

#[derive(Parser)]
/// Print JSON storage key-value pairs (as base64) for `StateInit`
/// of a Outlayer App contract
struct Args {
    /// Admin account ID (controls code approval and configuration)
    #[arg(long, value_name = "AccountId")]
    admin_id: AccountId,

    /// URL where the code binary can be fetched from
    /// (e.g. `https://...` or `data:application/wasm;base64,...`)
    #[arg(long, value_name = "URL")]
    code_url: Url,

    /// SHA-256 hash of the approved code.
    ///
    /// `HASH` can be encoded as base58 or hex with `0x` prefix.
    /// `@FILE` will calculate SHA-256 hash of the `FILE` contents.
    /// `@-` will calculate SHA-256 hash of the stdin contents.
    #[arg(long, value_name = "HASH | @FILE | @-")]
    code_hash: HashSource<Sha256>,

    /// Admin's public key (e.g. `ed25519:...`)
    #[arg(long, value_name = "PublicKey")]
    admin_public_key: AdminPublicKey,

    /// Output single-line JSON only (no human-readable annotations)
    #[arg(short, long)]
    quiet: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let code_hash = args.code_hash.hash().context("code_hash")?;

    let state = State::new(args.admin_id, args.admin_public_key)
        .with_code_hash(code_hash)
        .with_code_url(args.code_url.to_string());

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
