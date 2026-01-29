use anyhow::Result;
use clap::{Parser, Subcommand};
use xtask::{BuildOptions, Contract, build_contract, build_workspace_contracts};

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
            let path = build_workspace_contracts(&options)?;
            println!("Built all contracts at: {:?}", path);
        }
        Commands::Build { contract, options } => {
            let path = build_contract(&contract, &options)?;
            println!("Built {:?} contract at: {:?}", contract, path);
        }
    }

    Ok(())
}
