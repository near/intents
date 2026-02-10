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
            let contracts = Contract::all()
                .into_iter()
                .map(|c| (c, options.clone()))
                .collect();

            build_contracts(contracts)?
        }
        Commands::Build { contract, options } => build_contracts(vec![(contract, options)])?,
    };

    cargo_warning!("Built {} contracts", artifacts.len());

    for a in artifacts {
        cargo_warning!("Built {:?} at: {:?}", a.contract, a.wasm_path);
    }

    Ok(())
}
