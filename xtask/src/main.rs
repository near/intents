use anyhow::Result;
use clap::{Parser, Subcommand};
use xtask::{BuildOptions, Contract, ContractOptions, build_contracts};

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
            build_contracts(ContractOptions::all_without_features(), options)?
        }
        Commands::Build { contract, options } => build_contracts(
            vec![ContractOptions::new_without_features(contract)],
            options,
        )?,
    };

    println!("Built {} contracts", artifacts.len());

    for a in artifacts {
        println!("Built {:?} at: {:?}", a.contract, a.wasm_path);
    }

    Ok(())
}
