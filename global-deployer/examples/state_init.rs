use clap::Parser;
use defuse_global_deployer::State;
use near_sdk::{AccountId, base64::prelude::*};

#[derive(Parser)]
#[command(about = "Compute StateInit for a global-deployer contract")]
struct Args {
    /// Owner account ID
    #[arg(long)]
    owner: AccountId,

    /// Hex-encoded 32-byte code hash
    #[arg(long)]
    code_hash: String,

    /// Hex-encoded 32-byte approved hash
    #[arg(long)]
    approved_hash: String,

    /// Output single-line JSON with base64-encoded keys/values
    #[arg(long)]
    quiet: bool,
}

fn main() {
    let args = Args::parse();

    let code_hash: [u8; 32] = hex::decode(args.code_hash.strip_prefix("0x").unwrap_or(&args.code_hash))
        .expect("invalid code_hash hex")
        .try_into()
        .expect("code_hash must be 32 bytes");
    let approved_hash: [u8; 32] =
        hex::decode(args.approved_hash.strip_prefix("0x").unwrap_or(&args.approved_hash))
            .expect("invalid approved_hash hex")
            .try_into()
            .expect("approved_hash must be 32 bytes");

    let state = State {
        owner_id: args.owner,
        code_hash,
        approved_hash,
    };

    let state_init = state.state_init();

    if args.quiet {
        let map: serde_json::Map<String, serde_json::Value> = state_init
            .iter()
            .map(|(k, v)| {
                (
                    BASE64_STANDARD.encode(k),
                    serde_json::Value::String(BASE64_STANDARD.encode(v)),
                )
            })
            .collect();
        println!("{}", serde_json::to_string(&map).unwrap());
    } else {
        println!(
            "State:\n  owner_id: {}\n  code_hash: 0x{}\n  approved_hash: 0x{}",
            state.owner_id,
            hex::encode(state.code_hash),
            hex::encode(state.approved_hash),
        );

        println!("\nStateInit:");
        println!("{{");
        for (key, value) in &state_init {
            println!(
                "  \"{}\": \"{}\"",
                BASE64_STANDARD.encode(key),
                BASE64_STANDARD.encode(value),
            );
        }
        println!("}}");
    }
}
