use clap::Parser;
use defuse_global_deployer::State;
use near_sdk::{AccountId, base64::prelude::*, bs58, serde_json};
use std::collections::BTreeMap;

fn parse_hex_hash(s: &str) -> Result<[u8; 32], String> {
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

#[derive(Parser)]
#[command(about = "Compute StateInit for a global-deployer contract")]
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
    #[arg(long, value_parser = parse_hex_hash, value_name = "HASH")]
    approve: Option<[u8; 32]>,

    /// Output single-line JSON with base64-encoded keys/values
    #[arg(short, long)]
    quiet: bool,
}

fn main() {
    let args = Args::parse();

    let mut code_hash = [0u8; 32];
    code_hash[32 - 4..].copy_from_slice(&args.index.to_be_bytes());
    let approved_hash = args.approve.unwrap_or([0u8; 32]);

    let state = State {
        owner_id: args.owner_id,
        code_hash,
        approved_hash,
    };

    let state_init = state.state_init();

    if !args.quiet {
        eprintln!("{:<15} {}", "owner_id:", state.owner_id);
        eprintln!("{:<15} {}", "code_hash:", hex::encode(state.code_hash));
        eprintln!(
            "{:<15} {}",
            "approved_hash:",
            hex::encode(state.approved_hash),
        );
    }

    let map = state_init
        .iter()
        .map(|(k, v)| (BASE64_STANDARD.encode(k), BASE64_STANDARD.encode(v)))
        .collect::<BTreeMap<_, _>>();
    println!("{}", serde_json::to_string(&map).unwrap());
}
