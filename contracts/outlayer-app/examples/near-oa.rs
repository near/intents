use clap::Parser;
use defuse_outlayer_app::{State, Url};
use near_sdk::{AccountId, base64::prelude::*, serde_json};
use std::collections::BTreeMap;

fn parse_hex_hash(s: &str) -> Result<[u8; 32], String> {
    if let Some(s) = s.strip_prefix("0x") {
        hex::decode(s).map_err(|err| format!("hex: {err}"))?
    } else {
        hex::decode(s).map_err(|err| format!("hex: {err}"))?
    }
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
    code_url: String,

    /// Pre-approve a SHA-256 code hash (hex, with or without 0x prefix).
    #[arg(long, value_parser = parse_hex_hash, value_name = "HASH")]
    approve: Option<[u8; 32]>,

    /// Output single-line JSON only (no human-readable annotations)
    #[arg(short, long)]
    quiet: bool,
}

fn main() {
    let args = Args::parse();

    let url = Url::parse(&args.code_url).unwrap_or_else(|e| panic!("invalid --code-url: {e}"));
    let code_hash = args.approve.unwrap_or([0u8; 32]);
    let state = State::new(args.admin_id.clone(), code_hash, url);

    if !args.quiet {
        eprintln!("{:<20} {}", "admin_id:", state.admin_id);
        eprintln!("{:<20} {}", "code_hash:", hex::encode(state.code_hash));
        eprintln!("{:<20} {}", "code_url:", state.code_url.0);
    }

    let state_init = state.state_init();
    let map = state_init
        .iter()
        .map(|(k, v)| (BASE64_STANDARD.encode(k), BASE64_STANDARD.encode(v)))
        .collect::<BTreeMap<_, _>>();
    println!("{}", serde_json::to_string(&map).unwrap());
}
