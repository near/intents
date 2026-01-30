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

    match cli.command {
        Commands::BuildAll(options) => {
            let path = build_contracts(ContractOptions::all_without_features(), options)?;
            println!("Built all contracts at: {:?}", path);
        }
        Commands::Build { contract, options } => {
            let results = build_contracts(
                vec![ContractOptions::new_without_features(contract)],
                options,
            )?;

            let (contract, path) = results
                .first()
                .ok_or(anyhow::anyhow!("No contracts built"))?;

            println!("Built {:?} contract at: {:?}", contract, path);
        }
    }

    Ok(())
}
