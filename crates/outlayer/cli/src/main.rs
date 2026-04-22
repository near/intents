use anyhow::Result;
use clap::Parser;
use std::io::IsTerminal;
use std::path::PathBuf;
use wasmtime_wasi::cli::{InputFile, StdinStream};

use defuse_outlayer_state::HostState;
use defuse_outlayer_vm_runner::{Context, VmRuntime};

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
    /// Path to the WebAssembly component to execute
    path: PathBuf,

    /// Optional path to a file whose contents will be passed to the components stdin
    #[clap(long)]
    input_file: Option<PathBuf>,

    /// Maximum number of WebAssembly instructions the component may execute
    #[clap(long, short)]
    fuel_limit: Option<u64>,

    /// Maximum memory the component may use, in bytes
    #[clap(long, short)]
    memory_limit: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let stdin: Box<dyn StdinStream + Sync> = match args.input_file {
        Some(path) => Box::new(InputFile::new(std::fs::File::open(path)?)),
        None if !std::io::stdin().is_terminal() => Box::new(std::io::stdin()),
        None => Box::new(std::io::empty()),
    };

    let ctx = Context::new(
        stdin,
        std::io::stdout(),
        std::io::stderr(),
        HostState::default(),
    );

    let mut builder = VmRuntime::<HostState>::builder();
    if let Some(fuel_limit) = args.fuel_limit {
        builder = builder.fuel_limit(fuel_limit);
    }
    if let Some(memory_limit) = args.memory_limit {
        builder = builder.memory_limit(memory_limit);
    }
    let runner = builder.build()?;

    let wasm_binary = std::fs::read(args.path)?;
    let component = runner.compile(&wasm_binary)?;

    runner.execute(ctx, &component).await?;

    Ok(())
}
