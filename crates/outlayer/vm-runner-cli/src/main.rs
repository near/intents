use anyhow::{Context, Result};
use bytesize::ByteSize;
use clap::Parser;
use std::{borrow::Cow, env, path::PathBuf};

use defuse_outlayer_vm_runner::{
    Context as RunnerContext, VmRuntime,
    host::{Context as HostContext, InMemorySigner, State, primitives::AppId},
};

#[derive(Parser)]
#[command(
    about = "Execute a WASI component with a custom host environment",
    long_about = "\n
    Runs a WASIP2 component, wiring its stdin/stdout/stderr to the host \n
    Input can be piped from stdin or supplied via --input-file.\n
    Execution is bounded by configurable fuel and memory limits.
    "
)]
struct Args {
    /// Application ID to use in the host context
    app_id: AppId<'static>,

    /// Path to the WebAssembly component to execute
    path: PathBuf,

    /// Maximum number of WebAssembly instructions the component may execute
    #[clap(long, short)]
    fuel: Option<u64>,

    /// Maximum memory the component may use, in bytes
    #[clap(long, short)]
    memory: Option<ByteSize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let seed = hex::decode(env::var("OUTLAYER_SEED").context("OUTLAYER_SEED env var is not set")?)
        .context("OUTLAYER_SEED must be a hex-encoded byte string")?;

    let state = State::new(
        HostContext {
            app_id: args.app_id,
        },
        Cow::Owned(InMemorySigner::from_seed(&seed)),
    );

    let ctx = RunnerContext::new(
        tokio::io::stdin(),
        tokio::io::stdout(),
        tokio::io::stderr(),
        state,
    );

    let mut builder = VmRuntime::<State>::builder();

    if let Some(fuel) = args.fuel {
        builder = builder.fuel_limit(fuel);
    }

    if let Some(memory) = args.memory {
        builder = builder.memory_limit(memory.as_u64().try_into().with_context(|| {
            format!(
                "memory limit {memory} exceeds platform maximum ({} bytes)",
                usize::MAX
            )
        })?);
    }
    let runner = builder.build().context("runtime: build")?;

    let wasm_binary = std::fs::read(&args.path)
        .with_context(|| format!("failed to read: {}", args.path.display()))?;

    let component = runner
        .compile(&wasm_binary)
        .with_context(|| format!("failed to compile: {}", args.path.display()))?;

    let outcome = runner.execute(ctx, &component).await.context("execute")?;

    if let Some(error) = outcome.error {
        eprintln!("{error:?}");
    }

    Ok(())
}
