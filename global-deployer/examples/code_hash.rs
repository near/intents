use std::{env, fs};

use near_sdk::{borsh, bs58};
use sha2::{Digest, Sha256};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let data = fs::read(input_path).expect("failed to read file");
    println!("File: {input_path}");
    println!("Size: {} bytes", data.len());

    let hash: [u8; 32] = Sha256::digest(&data).into();
    println!("\nsha256:");
    println!("  hex:    0x{}", hex::encode(hash));
    println!("  base58: {}", bs58::encode(hash).into_string());

    let borsh_code: Vec<u8> = borsh::to_vec(&data).expect("borsh serialization failed");
    let output_path = format!("{input_path}.borsh");
    fs::write(&output_path, &borsh_code).expect("failed to write borsh file");
    println!("\nBorsh-serialized Vec<u8> (for gd_deploy):");
    println!("  size:   {} bytes", borsh_code.len());
    println!("  output: {output_path}");
}
