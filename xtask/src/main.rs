use anyhow::Result;
use clap::{Parser, Subcommand};
use xtask::{BuildOptions, Contract, build_contracts, cargo_warning};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build {
        contract: Contract,
        #[command(flatten)]
        options: BuildOptions,
    },
    BuildAll(BuildOptions),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let artifacts = match cli.command {
        Commands::BuildAll(options) => {
            build_contracts(Contract::all().into_iter().map(|c| (c, options.clone())))?
        }

        Commands::Build { contract, options } => build_contracts(vec![(contract, options)])?,
    };

    cargo_warning!("xtask build: built {} contracts", artifacts.len());

    for a in artifacts {
        cargo_warning!("xtask build: built {:?} at: {:?}", a.contract, a.wasm_path);
    }

    Ok(())
}
