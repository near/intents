use std::sync::Arc;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine};
use bytes::Bytes;
use clap::Parser;
use defuse_outlayer_service::{
    build_stack,
    types::{AccountId, OnChainRequest, Request, WorkerSigningKey},
};
use defuse_outlayer_state::HostState;
use defuse_outlayer_vm_runner::VmRuntime;
use ed25519_dalek::SigningKey;
use tower::ServiceExt;
use uuid::Uuid;

/// Run a WASM component through the outlayer service stack.
///
/// Examples:
///   # inline
///   run 'data:application/wasm;base64,AGFzbQ...' --string-input 'hello world'
///
///   # remote (hash must be provided)
///   run 'https://example.com/component.wasm' --string-input 'hello' --wasm-hash <64 hex chars>
#[derive(Parser)]
#[command(group(clap::ArgGroup::new("input").required(true).args(["string_input", "base64_input"])))]
struct Args {
    /// WASM URL — inline (`data:application/wasm;base64,…`) or remote (`http(s)://…`).
    url: String,

    /// Input passed to the component via stdin as a UTF-8 string.
    #[arg(long)]
    string_input: Option<String>,

    /// Input passed to the component via stdin, decoded from base64.
    #[arg(long)]
    base64_input: Option<String>,

    /// SHA-256 of the WASM binary (64 hex chars).
    #[arg(long)]
    wasm_hash: String,
}



fn parse_hash_hex(hex: &str) -> anyhow::Result<[u8; 32]> {
    if hex.len() != 64 {
        anyhow::bail!("--wasm-hash must be 64 hex chars (got {})", hex.len());
    }
    let mut out = [0u8; 32];
    for (i, pair) in hex.as_bytes().chunks(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(pair)?, 16)
            .map_err(|_| anyhow::anyhow!("invalid hex digit in --wasm-hash"))?;
    }
    Ok(out)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let wasm_hash = parse_hash_hex(&args.wasm_hash)?;

    let request = Request::OnChain(OnChainRequest {
        id: Uuid::new_v4(),
        tx_hash: [0u8; 32],
        project_id: AccountId("example.near".to_string()),
        input: if let Some(s) = args.string_input {
            Bytes::copy_from_slice(s.as_bytes())
        } else if let Some(b) = args.base64_input {
            Bytes::from(STANDARD.decode(b)?)
        } else {
            unreachable!("clap enforces exactly one of --string-input / --base64-input")
        },
        wasm_hash,
        wasm_url: args.url,
    });

    // Fixed key — replace with a real TEE key in production.
    let signing_key = WorkerSigningKey(Arc::new(SigningKey::from_bytes(&[1u8; 32])));
    let runtime = Arc::new(VmRuntime::<HostState>::new()?);

    let signed = build_stack(
        signing_key,
        Duration::from_secs(30),
        Duration::from_secs(10),
        Duration::from_secs(5),
        2,
        runtime,
    )
    .oneshot(request)
    .await?;

    println!("{:#?}", signed);

    Ok(())
}
