use std::borrow::Cow;
use std::io::Read as _;
use std::io::Write as _;
use std::sync::Arc;

use bytes::Bytes;
use clap::Parser;
use defuse_outlayer_host::primitives::{AccountIdRef, AppId};
use defuse_outlayer_host::{Context, InMemorySigner, State};
use defuse_outlayer_service::{
    Config, ExecutionRequest, ExecutionStackError, Request, build_stack,
    types::{AccountId, OffChainRequest},
};
use defuse_outlayer_vm_runner::VmRuntime;
use tower::{ServiceExt, service_fn};

/// Run a WASM component through the outlayer service stack.
///
/// Input is read from stdin; the component's stdout/stderr are forwarded to
/// the host's stdout/stderr.
///
/// Examples:
///   echo 'hello world' | run 'data:application/wasm;base64,AGFzbQ...' --wasm-hash <64 hex chars>
///   run '<https://example.com/component.wasm>' --wasm-hash <64 hex chars> < input.bin
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
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let wasm_hash = parse_hash_hex(&args.wasm_hash)?;
    let wasm_url = args.url.clone();

    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input)?;

    let request = Request::OffChain(OffChainRequest {
        request_id: "dummy_id".to_string(),
        project_id: AccountId("example.near".to_string()),
        input: Bytes::from(input),
    });

    // TODO: replace with OnChainFetchService once NEAR RPC fetch is implemented
    let fetch = service_fn(move |req: OffChainRequest| {
        let url = wasm_url.clone();
        async move {
            Ok::<ExecutionRequest, ExecutionStackError>(ExecutionRequest {
                request_id: req.request_id,
                project_id: req.project_id,
                wasm_url: url,
                wasm_hash,
                input: req.input,
            })
        }
    });

    // Fixed key — replace with a real TEE key in production.
    let signing_key = defuse_outlayer_crypto::signer::InMemorySigner::from_seed(&[1u8; 32]);
    let host_template = State::new(
        Context {
            app_id: AppId::Near(Cow::Borrowed(AccountIdRef::new_or_panic("example.near"))),
        },
        Cow::Owned(InMemorySigner::from_seed(b"example")),
    );
    let runtime = Arc::new(VmRuntime::<State<'static>>::new()?);

    let signed = build_stack(signing_key, runtime, Config::default(), host_template, fetch)
        .oneshot(request)
        .await?;

    if args.verbose {
        eprintln!("{}", serde_json::to_string_pretty(&signed)?);
    }

    std::io::stderr().write_all(&signed.response.logs)?;
    std::io::stdout().write_all(&signed.response.result?)?;

    Ok(())
}
