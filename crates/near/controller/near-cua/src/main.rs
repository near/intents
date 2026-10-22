use std::io::{Read, Write, stdin, stdout};

use anyhow::Context;
use base64::engine::general_purpose;
use clap::Parser;
use defuse_controller::UpgradeArgs;
use near_gas::NearGas as Gas;
use sha2::{Digest, Sha256};

/// Prepare Borsh-serialized arguments to pass into `upgrade()` method.
/// Reads code from stdin and writes final arguments to stdout.
#[derive(Parser)]
struct Args {
    /// Gas to pass to `state_migrate` method, e.g. `15 TGas`.
    #[arg(long, value_name = "GAS")]
    state_migration_gas: Option<Gas>,

    /// Write arguments to stdout as base64 encoded.
    #[arg(long)]
    base64: bool,

    /// Do not print code hashes to stderr.
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

    let update_args = UpgradeArgs {
        code: &code,
        state_migration_gas: args.state_migration_gas,
    };

    if !args.quiet {
        let code_hash = Sha256::digest(update_args.code);
        eprintln!("SHA-256 code hash (hex):    {}", hex::encode(code_hash));
        eprintln!(
            "SHA-256 code hash (base58): {}",
            bs58::encode(code_hash).into_string()
        );
    }

    let mut writer = stdout().lock();
    if args.base64 {
        let mut writer = base64::write::EncoderWriter::new(&mut writer, &general_purpose::STANDARD);
        borsh::to_writer(&mut writer, &update_args).context("borsh")?;
        writer.finish().context("base64")?;
    } else {
        borsh::to_writer(&mut writer, &update_args).context("borsh")?;
    }
    writer.flush().context("stdout")
}
