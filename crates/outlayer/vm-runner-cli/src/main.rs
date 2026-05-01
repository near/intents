use anyhow::{Context, Result};
use bytesize::ByteSize;
use clap::Parser;
use std::{borrow::Cow, path::PathBuf};

// Generated via near-cli@0.26.0:
// ```sh
// near contract state-init \
//   use-global-account-id 'test' \
//   data-from-json "$(near oa -q \
//       --admin-id 'test' \
//       --code-hash '0000000000000000000000000000000000000000000000000000000000000000' \
//       --code-url 'data:' \
//   )" inspect account-id
// ```
const DEFAULT_APP_ID: &str = "near:0sab1c86e60758fe3e8fc7ae40ecd2df1a07513ca9";
const DEFAULT_FUEL: u64 = u64::MAX;

use defuse_outlayer_vm_runner::{
    ExecutionContext, VmRuntime,
    host::{AppContext, InMemorySigner, State, primitives::AppId},
};

#[derive(Parser)]
#[command(
    about = "Execute a WASI component with a custom host environment",
    long_about = "\n
    Runs a WASIP2 component, wiring its stdin/stdout/stderr to the host \n
    Input can be piped from stdin. Output is written to stdout and stderr.
    Execution is bounded by configurable fuel and memory limits.
    "
)]
struct Args {
    /// Path to the WebAssembly component to execute
    wasm_path: PathBuf,

    /// Path to a file containing the hex-encoded 32-byte seed for the host's signer key
    seed_path: PathBuf,

    /// Application ID to use in the host context
    #[clap(long, short, default_value = DEFAULT_APP_ID)]
    app_id: AppId<'static>,

    /// Maximum number of WebAssembly instructions the component may execute
    #[clap(long, short, default_value_t = DEFAULT_FUEL)]
    fuel: u64,

    /// Maximum memory the component may use, in bytes
    #[clap(long, short)]
    memory: Option<ByteSize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let seed_hex = std::fs::read_to_string(&args.seed_path)
        .with_context(|| format!("failed to read seed file: {}", args.seed_path.display()))?;
    let seed =
        hex::decode(seed_hex.trim()).context("seed file must contain a hex-encoded byte string")?;

    let state = State::new(
        AppContext {
            app_id: args.app_id,
        },
        Cow::Owned(InMemorySigner::from_seed(&seed)),
    );

    let mut ctx = ExecutionContext::new(
        tokio::io::stdin(),
        tokio::io::stdout(),
        tokio::io::stderr(),
        state,
    )
    .fuel_limit(args.fuel);

    if let Some(memory) = args.memory {
        ctx = ctx.memory_limit(memory.as_u64().try_into().with_context(|| {
            format!(
                "memory limit {memory} exceeds platform maximum ({} bytes)",
                usize::MAX
            )
        })?);
    }

    let runner = VmRuntime::<State>::new().context("failed to initialize runtime")?;

    let wasm_binary = std::fs::read(&args.wasm_path)
        .with_context(|| format!("failed to read: {}", args.wasm_path.display()))?;

    let component = runner
        .compile(&wasm_binary)
        .with_context(|| format!("failed to compile: {}", args.wasm_path.display()))?;

    let outcome = runner.execute(ctx, &component).await.context("execute")?;

    outcome.into_result().context("component failed")
}
