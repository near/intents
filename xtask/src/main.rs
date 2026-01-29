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
        #[arg(short, long)]
        contract: Contract,
        #[command(flatten)]
        options: BuildOptions,
    },
    BuildAll(BuildOptions),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::BuildAll(options) => build_workspace_contracts(&options),
        Commands::Build { contract, options } => build_contract(contract, &options),
    }
}
