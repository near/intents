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
            let artifacts = build_contracts(ContractOptions::all_without_features(), options)?;
            println!("Built {} contracts", artifacts.len());

            for a in artifacts {
                println!("Built {:?} at: {:?}", a.contract, a.wasm_path);
                if let Some(checksum) = a.checksum_hex {
                    println!("Checksum: {}", checksum);
                }
            }
        }
        Commands::Build { contract, options } => {
            let results = build_contracts(
                vec![ContractOptions::new_without_features(contract)],
                options,
            )?;

            let a = results
                .first()
                .ok_or(anyhow::anyhow!("No contracts built"))?;

            println!("Built {:?} contract at: {:?}", a.contract, a.wasm_path);
            if let Some(checksum) = &a.checksum_hex {
                println!("Checksum: {}", checksum);
            }
        }
    }

    Ok(())
}
