use clap::Parser;
use defuse_outlayer_project::State;
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
#[command(about = "Compute StateInit for an outlayer-project contract instance")]
struct Args {
    /// Updater account ID (controls WASM approval and env vars)
    #[arg(long, value_name = "AccountId")]
    updater_id: AccountId,

    /// Pre-approve a SHA-256 WASM hash (hex, with or without 0x prefix).
    /// When set, the first `oc_upload_code()` won't require a prior `oc_approve()`.
    #[arg(long, value_parser = parse_hex_hash, value_name = "HASH")]
    approve: Option<[u8; 32]>,

    /// Output single-line JSON only (no human-readable annotations)
    #[arg(short, long)]
    quiet: bool,
}

fn main() {
    let args = Args::parse();

    let mut state = State::new(args.updater_id.clone());
    if let Some(hash) = args.approve {
        state = state.pre_approve(hash);
    }

    if !args.quiet {
        eprintln!("{:<20} {}", "updater_id:", state.updater_id);
        eprintln!(
            "{:<20} {}",
            "wasm_hash:",
            hex::encode(state.wasm_hash)
        );
    }

    let state_init = state.state_init();
    let map = state_init
        .iter()
        .map(|(k, v)| (BASE64_STANDARD.encode(k), BASE64_STANDARD.encode(v)))
        .collect::<BTreeMap<_, _>>();
    println!("{}", serde_json::to_string(&map).unwrap());
}
