//! BIP-322 signature verification utilities
//!
//! This module provides utility functions for address validation and public key
//! verification used in BIP-322 signature validation.

use crate::bitcoin_minimal::{Address, OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160};
use defuse_crypto::{Curve, Secp256k1};
use digest::Digest;
use near_sdk::env;

/// Validates that a recovered public key matches the expected Bitcoin address.
///
/// This function performs address-specific validation for all supported Bitcoin address types.
///
/// # Arguments
///
/// * `recovered_pubkey` - The 64-byte raw public key recovered from signature
/// * `address` - The Bitcoin address to validate against
///
/// # Returns
///
/// `true` if the public key matches the address, `false` otherwise
pub fn validate_pubkey_matches_address(
    recovered_pubkey: &<Secp256k1 as Curve>::PublicKey,
    address: &Address,
) -> bool {
    match address {
        Address::P2PKH { pubkey_hash } => validate_p2pkh_address(recovered_pubkey, pubkey_hash),
        Address::P2WPKH { witness_program } => {
            validate_p2wpkh_address(recovered_pubkey, witness_program)
        }
        Address::P2SH { script_hash } => validate_p2sh_address(recovered_pubkey, script_hash),
        Address::P2WSH { witness_program } => {
            validate_p2wsh_address(recovered_pubkey, witness_program)
        }
    }
}

/// Validates that a compressed public key matches the expected Bitcoin address.
///
/// This function performs address-specific validation using the compressed public key format
/// directly, without requiring decompression to the uncompressed format.
///
/// # Arguments
///
/// * `compressed_pubkey` - The 33-byte compressed public key
/// * `address` - The Bitcoin address to validate against
///
/// # Returns
///
/// `true` if the compressed public key matches the address, `false` otherwise
pub fn validate_compressed_pubkey_matches_address(
    compressed_pubkey: &[u8; 33],
    address: &Address,
) -> bool {
    match address {
        Address::P2PKH { pubkey_hash } => {
            let computed_hash: [u8; 20] =
                defuse_near_utils::digest::Hash160::digest(compressed_pubkey).into();
            computed_hash == *pubkey_hash
        }
        Address::P2WPKH { witness_program } => {
            let computed_hash: [u8; 20] =
                defuse_near_utils::digest::Hash160::digest(compressed_pubkey).into();
            computed_hash == witness_program.program.as_slice()
        }
        Address::P2SH { script_hash } => {
            let pubkey_hash: [u8; 20] =
                defuse_near_utils::digest::Hash160::digest(compressed_pubkey).into();

            // For P2SH-P2WPKH (nested segwit), create a P2WPKH witness program
            // Format: [version_byte][20_byte_pubkey_hash]
            let mut witness_program = Vec::with_capacity(22);
            witness_program.push(0x00); // witness version 0
            witness_program.push(0x14); // 20 bytes length
            witness_program.extend_from_slice(&pubkey_hash);

            let computed_script_hash: [u8; 20] =
                defuse_near_utils::digest::Hash160::digest(&witness_program).into();
            computed_script_hash == *script_hash
        }
        Address::P2WSH { witness_program } => {
            let pubkey_hash: [u8; 20] =
                defuse_near_utils::digest::Hash160::digest(compressed_pubkey).into();
            let witness_script = build_script(&pubkey_hash);
            let computed_script_hash = env::sha256_array(&witness_script);
            computed_script_hash == witness_program.program.as_slice()
        }
    }
}

/// Computes hash160 of a raw public key using the appropriate Bitcoin format.
///
/// # Arguments
///
/// * `raw_pubkey` - The raw public key from ecrecover (always 64 bytes)
/// * `compressed` - Whether to use compressed (true) or uncompressed (false) format
///
/// # Returns
///
/// The 20-byte hash160 result, or the input hash if not 64 bytes
fn hash160_pubkey(raw_pubkey: &[u8; 64], compressed: bool) -> Vec<[u8; 20]> {
    if compressed {
        // Since pubkey is restored, we don't know which (odd or even) y was used to
        // build compressed key and calculate the hash.
        // It means that we have to calculate hash for both possibilities.
        let mut compressed = Vec::with_capacity(33);
        compressed.push(0x02);
        compressed.extend_from_slice(&raw_pubkey[..32]);

        let mut response = Vec::with_capacity(2);
        response.push(defuse_near_utils::digest::Hash160::digest(&compressed).into());

        compressed.as_mut_slice()[0] = 0x03;
        response.push(defuse_near_utils::digest::Hash160::digest(&compressed).into());

        return response;
    }

    vec![defuse_near_utils::digest::Hash160::digest(raw_pubkey).into()]
}

/// Assemble witness or redeem script
///
/// # Arguments
///
/// * `pubkey_hash` - The HASH160 of the public key
///
/// # Returns
///
/// Assembled script which verifies given hash
fn build_script(pubkey_hash: &[u8; 20]) -> Vec<u8> {
    let mut script = Vec::with_capacity(25);
    script.push(OP_DUP);
    script.push(OP_HASH160);
    script.push(20);
    script.extend_from_slice(pubkey_hash);
    script.push(OP_EQUALVERIFY);
    script.push(OP_CHECKSIG);
    script
}

/// Validates a P2PKH address against a recovered public key.
fn validate_p2pkh_address(recovered_pubkey: &[u8; 64], expected_pubkey_hash: &[u8; 20]) -> bool {
    // Try uncompressed first
    let uncompressed_hash = hash160_pubkey(recovered_pubkey, false);
    if uncompressed_hash[0] == *expected_pubkey_hash {
        return true;
    }

    // Try compressed next, two possibilities
    let compressed_hash = hash160_pubkey(recovered_pubkey, true);
    compressed_hash[0] == *expected_pubkey_hash || compressed_hash[1] == *expected_pubkey_hash
}

/// Validates a P2WPKH address against a recovered public key.
fn validate_p2wpkh_address(
    recovered_pubkey: &[u8; 64],
    witness_program: &crate::bitcoin_minimal::WitnessProgram,
) -> bool {
    // P2WPKH addresses always use compressed public keys, so two possibilities,
    // depending on the y coordinate parity
    let computed_pubkey_hash = hash160_pubkey(recovered_pubkey, true);

    computed_pubkey_hash[0] == witness_program.program.as_slice()
        || computed_pubkey_hash[1] == witness_program.program.as_slice()
}

/// Validates a P2SH address against a recovered public key.
fn validate_p2sh_address(recovered_pubkey: &[u8; 64], expected_script_hash: &[u8; 20]) -> bool {
    // Try uncompressed first
    let pubkey_hash = hash160_pubkey(recovered_pubkey, false);
    let redeem_script = build_script(&pubkey_hash[0]);
    let computed_script_hash: [u8; 20] =
        defuse_near_utils::digest::Hash160::digest(&redeem_script).into();

    if computed_script_hash == *expected_script_hash {
        return true;
    }

    // Try compressed next, two possibilities
    let pubkey_hash = hash160_pubkey(recovered_pubkey, true);

    let redeem_script = build_script(&pubkey_hash[0]);
    let computed_script_hash: [u8; 20] =
        defuse_near_utils::digest::Hash160::digest(&redeem_script).into();
    if computed_script_hash == *expected_script_hash {
        return true;
    }

    let redeem_script = build_script(&pubkey_hash[1]);
    let computed_script_hash: [u8; 20] =
        defuse_near_utils::digest::Hash160::digest(&redeem_script).into();
    computed_script_hash == *expected_script_hash
}

/// Validates a P2WSH address against a recovered public key.
fn validate_p2wsh_address(
    recovered_pubkey: &[u8; 64],
    witness_program: &crate::bitcoin_minimal::WitnessProgram,
) -> bool {
    // Try uncompressed first
    let pubkey_hash = hash160_pubkey(recovered_pubkey, false);
    let witness_script = build_script(&pubkey_hash[0]);
    let computed_script_hash = env::sha256_array(&witness_script);

    if computed_script_hash == witness_program.program.as_slice() {
        return true;
    }

    // Try compressed next
    let pubkey_hash = hash160_pubkey(recovered_pubkey, true);

    let witness_script = build_script(&pubkey_hash[0]);
    let computed_script_hash = env::sha256_array(&witness_script);
    if computed_script_hash == witness_program.program.as_slice() {
        return true;
    }

    let witness_script = build_script(&pubkey_hash[1]);
    let computed_script_hash = env::sha256_array(&witness_script);
    computed_script_hash == witness_program.program.as_slice()
}
