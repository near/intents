use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use xtask::{
    DefuseContract, EscrowSwapContract, MultiTokenReceiverStubContract, PoaFactoryContract,
    PoaTokenContract, build_contract, build_workspace_contracts,
};

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

#[derive(Args, Clone)]
struct BuildOptions {
    #[arg(short, long)]
    reproducible: Option<bool>,
    #[arg(short, long, default_value_t = false)]
    checksum: bool,
    #[arg(short, long)]
    features: Option<String>,
    #[arg(short, long)]
    env_var: Option<String>,
    #[arg(short, long)]
    outdir: Option<String>,
}

#[derive(ValueEnum, Clone)]
enum Contract {
    Defuse,
    PoaFactory,
    PoaToken,
    EscrowSwap,
    MultiTokenReceiverStub,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::BuildAll(options) => {
            build_workspace_contracts(&options.reproducible, &options.outdir)
        }
        Commands::Build { contract, options } => match contract {
            Contract::Defuse => {
                build_contract::<DefuseContract>(&options.reproducible, &options.outdir)
            }
            Contract::PoaFactory => {
                build_contract::<PoaFactoryContract>(&options.reproducible, &options.outdir)
            }
            Contract::PoaToken => {
                build_contract::<PoaTokenContract>(&options.reproducible, &options.outdir)
            }
            Contract::EscrowSwap => {
                build_contract::<EscrowSwapContract>(&options.reproducible, &options.outdir)
            }
            Contract::MultiTokenReceiverStub => build_contract::<MultiTokenReceiverStubContract>(
                &options.reproducible,
                &options.outdir,
            ),
        },
    }
}
