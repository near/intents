use std::{borrow::Cow, path::PathBuf};

use anyhow::{Context, Result};
use bytesize::ByteSize;
use clap::Parser;
use tokio::{fs, io};

use defuse_outlayer_vm_runner::{
    Context, VmRuntime,
    host::{
        AppContext, InMemorySigner, State,
        primitives::{AccountIdRef, AppId},
    },
};

// Generated via near-cli@0.26.1:
// ```sh
// near contract state-init \
//   use-global-account-id 'test' \
//   data-from-json "$(near oa -q \
//       --admin-id 'test' \
//       --code-hash '0000000000000000000000000000000000000000000000000000000000000000' \
//       --code-url 'data:application/wasm;base64,' \
//   )" inspect account-id
// ```
// matches mocked default in SDK
const DEFAULT_APP_ID: AppId = AppId::Near(Cow::Borrowed(AccountIdRef::new_or_panic(
    "0se1573c9dff58d4a57384dee048c9b1a809fb6839",
)));

// matches mocked default in SDK
const DEFAULT_SEED: &[u8] = b"test";

const DEFAULT_FUEL: u64 = u64::MAX;

#[derive(Parser)]
/// Execute a WASI-P2 component with a custom host environment
#[command(long_about = r#"
Run a WASI-p2 component, wiring its stdin/stdout/stderr to the host.
Input can be piped from stdin. Output is written to stdout and stderr.
Execution is bounded by configurable fuel and memory limits."#)]
struct Args {
    /// Path to the WebAssembly component to execute
    wasm: PathBuf,

    /// Application ID to use in the host context
    #[arg(long, short, default_value_t = DEFAULT_APP_ID)]
    app_id: AppId<'static>,

    /// Path to a file containing the raw seed for the host's signer key
    #[arg(long, value_name = "FILE")]
    seed: Option<PathBuf>,

    /// Maximum number of WebAssembly instructions the component may execute
    #[arg(long, default_value_t = DEFAULT_FUEL, value_name = "u64")]
    fuel: u64,

    /// Maximum memory the component may use
    #[arg(long, value_name = "MEM")]
    memory: Option<ByteSize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let seed = if let Some(seed) = args.seed {
        fs::read(&seed)
            .await
            .with_context(|| format!("seed: read {}", seed.display()))?
    } else {
        DEFAULT_SEED.to_vec()
    };

    let state = State::new(
        AppContext {
            app_id: args.app_id,
        },
        Cow::Owned(InMemorySigner::from_seed(&seed)),
    );

    let mut ctx = Context::new(
        io::stdin(),  // forward stdin
        io::stdout(), // forward stdout
        io::stderr(), // forward stderr
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

    let wasm_binary = fs::read(&args.wasm)
        .await
        .with_context(|| format!("failed to read: {}", args.wasm.display()))?;

    let runner = VmRuntime::<State>::new().context("failed to initialize runtime")?;

    let component = runner
        .compile(&wasm_binary)
        .with_context(|| format!("failed to compile: {}", args.wasm.display()))?;

    let outcome = runner.execute(ctx, &component).await.context("execute")?;

    outcome.into_result().map_err(Into::into)
}
