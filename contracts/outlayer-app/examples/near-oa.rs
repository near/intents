use clap::Parser;
use defuse_outlayer_app::State;
use near_sdk::{AccountId, base64::prelude::*, serde_json};
use std::collections::BTreeMap;
use url::Url;

fn parse_hex_hash(s: &str) -> Result<[u8; 32], String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s)
        .map_err(|err| format!("hex: {err}"))?
        .try_into()
        .map_err(|_| "hash must be 32 bytes encoded as hex (with or without 0x prefix)".to_string())
}

#[derive(Parser)]
#[command(about = "Compute StateInit for a near-oa contract instance")]
struct Args {
    /// Admin account ID (controls code approval and env vars)
    #[arg(long, value_name = "AccountId")]
    admin_id: AccountId,

    /// URL where the code binary can be fetched from
    /// (e.g. `https://...` or `data:application/wasm;base64,...`)
    #[arg(long, value_name = "URL")]
    code_url: Url,

    /// SHA-256 hash of the approved code (hex, with or without 0x prefix).
    /// Defaults to all-zeros if omitted.
    #[arg(long, value_parser = parse_hex_hash, value_name = "HASH")]
    code_hash: [u8; 32],

    /// Output single-line JSON only (no human-readable annotations)
    #[arg(short, long)]
    quiet: bool,
}

fn main() {
    let args = Args::parse();

    let state = State::new(
        args.admin_id.clone(),
        args.code_hash,
        args.code_url.to_string(),
    );

    if !args.quiet {
        eprintln!("{:<20} {}", "admin_id:", state.admin_id);
        eprintln!("{:<20} {}", "code_hash:", hex::encode(state.code_hash));
        eprintln!("{:<20} {}", "code_url:", state.code_url);
    }

    let state_init = state.state_init();
    let map = state_init
        .iter()
        .map(|(k, v)| (BASE64_STANDARD.encode(k), BASE64_STANDARD.encode(v)))
        .collect::<BTreeMap<_, _>>();
    println!("{}", serde_json::to_string(&map).unwrap());
}
