use std::io::{Read, stdin, stdout};

use anyhow::Context;
use base64::engine::general_purpose;
use clap::Parser;
use defuse_controller::UpgradeArgs;
use either::Either;
use near_gas::NearGas as Gas;
use sha2::{Digest, Sha256};

/// Prepare Borsh-serialized arguments to pass into `upgrade()` method.
/// Reads code from stdin and writes final arguments to stdout.
#[derive(Parser)]
struct Args {
    /// Gas to pass to `state_migrate` method, e.g. `15 TGas`
    #[arg(long, value_name = "GAS")]
    state_migration_gas: Option<Gas>,

    /// Write arguments as base64 encoded.
    #[arg(long)]
    base64: bool,

    /// Do not print code hashes to stderr
    #[arg(short, long)]
    quiet: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let code = {
        let mut buf = Vec::new();
        stdin()
            .read_to_end(&mut buf)
            .context("reading code from stdin")?;
        buf
    };

    if !args.quiet {
        let hash = Sha256::digest(&code);
        eprintln!("SHA-256 (hex): {}", hex::encode(hash));
        eprintln!("SHA-256 (base58): {}", bs58::encode(hash).into_string());
    }

    let update_args = UpgradeArgs {
        code: &code,
        state_migration_gas: args.state_migration_gas,
    };

    let writer = if args.base64 {
        Either::Left(base64::write::EncoderWriter::new(
            stdout(),
            &general_purpose::STANDARD,
        ))
    } else {
        Either::Right(stdout())
    };

    borsh::to_writer(writer, &update_args).context("borsh")
}
