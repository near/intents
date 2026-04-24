use std::io::Read as _;
use std::io::Write as _;
use std::sync::Arc;

use bytes::Bytes;
use clap::Parser;
use defuse_outlayer_service::{
    Config, WorkerSigningKey, build_stack,
    types::{AccountId, OnChainRequest, Request},
};
use defuse_outlayer_state::HostState;
use defuse_outlayer_vm_runner::VmRuntime;
use ed25519_dalek::SigningKey;
use tower::ServiceExt;

/// Run a WASM component through the outlayer service stack.
///
/// Input is read from stdin; the component's stdout/stderr are forwarded to
/// the host's stdout/stderr.
///
/// Examples:
///   echo 'hello world' | run 'data:application/wasm;base64,AGFzbQ...' --wasm-hash <64 hex chars>
///   run 'https://example.com/component.wasm' --wasm-hash <64 hex chars> < input.bin
#[derive(Parser)]
struct Args {
    /// WASM URL — inline (`data:application/wasm;base64,…`) or remote (`http(s)://…`).
    url: String,

    /// SHA-256 of the WASM binary (64 hex chars).
    #[arg(long)]
    wasm_hash: String,

    /// Print the full signed response (JSON + signature) to stderr.
    #[arg(long, short)]
    verbose: bool,
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

    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input)?;

    let request = Request::OnChain(OnChainRequest {
        tx_hash: [0u8; 32],
        project_id: AccountId("example.near".to_string()),
        input: Bytes::from(input),
        wasm_hash,
        wasm_url: args.url,
        nonce: [0u8; 32],
    });

    // Fixed key — replace with a real TEE key in production.
    let signing_key = WorkerSigningKey(Arc::new(SigningKey::from_bytes(&[1u8; 32])));
    let runtime = Arc::new(VmRuntime::<HostState>::new()?);

    let signed = build_stack(signing_key, runtime, Config::default())
        .oneshot(request)
        .await?;

    if args.verbose {
        eprintln!("{}", serde_json::to_string_pretty(&signed)?);
    }

    std::io::stderr().write_all(&signed.response.logs)?;
    std::io::stdout().write_all(&signed.response.result?)?;

    Ok(())
}
