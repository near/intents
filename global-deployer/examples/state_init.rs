use std::env;

use defuse_global_deployer::State;
use near_sdk::{AccountId, base64::prelude::*};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "Usage: {} <owner_id> <code_hash_hex> <approved_hash_hex>",
            args[0]
        );
        std::process::exit(1);
    }

    let owner_id: AccountId = args[1].parse().expect("invalid owner_id");
    let code_hash: [u8; 32] = hex::decode(args[2].strip_prefix("0x").unwrap_or(&args[2]))
        .expect("invalid code_hash hex")
        .try_into()
        .expect("code_hash must be 32 bytes");
    let approved_hash: [u8; 32] =
        hex::decode(args[3].strip_prefix("0x").unwrap_or(&args[3]))
            .expect("invalid approved_hash hex")
            .try_into()
            .expect("approved_hash must be 32 bytes");

    let state = State {
        owner_id,
        code_hash,
        approved_hash,
    };

    println!(
        "State:\n  owner_id: {}\n  code_hash: 0x{}\n  approved_hash: 0x{}",
        state.owner_id,
        hex::encode(state.code_hash),
        hex::encode(state.approved_hash),
    );

    let state_init = state.state_init();
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
