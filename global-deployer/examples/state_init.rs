use clap::Parser;
use defuse_global_deployer::State;
use near_sdk::{AccountId, base64::prelude::*};
use std::collections::BTreeMap;

fn parse_hex_hash(s: &str) -> Result<[u8; 32], String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).map_err(|e| format!("invalid hex: {e}"))?;
    bytes
        .try_into()
        .map_err(|_| "hash must be exactly 32 bytes".to_string())
}

#[derive(Parser)]
#[command(about = "Compute StateInit for a global-deployer contract")]
struct Args {
    /// Owner account ID
    #[arg(long)]
    owner: AccountId,

    /// Hex-encoded 32-byte code hash
    #[arg(long, value_parser = parse_hex_hash)]
    code_hash: [u8; 32],

    /// Hex-encoded 32-byte approved hash
    #[arg(long, value_parser = parse_hex_hash)]
    approved_hash: [u8; 32],

    /// Output single-line JSON with base64-encoded keys/values
    #[arg(long)]
    quiet: bool,
}

fn main() {
    let args = Args::parse();

    let state = State {
        owner_id: args.owner,
        code_hash: args.code_hash,
        approved_hash: args.approved_hash,
    };

    let state_init = state.state_init();

    if !args.quiet {
        println!("State:");
        println!("  {:<15} {}", "owner_id:", state.owner_id);
        println!("  {:<15} 0x{}", "code_hash:", hex::encode(state.code_hash));
        println!(
            "  {:<15} 0x{}",
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
