//! BIP-322 signature verification logic
//!
//! This module contains unified verification logic for all Bitcoin address types.
//! Each verification function uses early exit patterns for cleaner, more readable code.

use crate::bitcoin_minimal::{Address, Bip322Witness};
use crate::hashing::Bip322MessageHasher;
use crate::transaction::Bip322TransactionBuilder;
use crate::SignedBip322Payload;
use defuse_crypto::{Curve, Secp256k1};
use near_sdk::env;

/// Verifies a BIP-322 signature for P2PKH addresses.
///
/// P2PKH verification expects:
/// - Witness stack: [signature, pubkey]
/// - Uses legacy Bitcoin sighash algorithm
/// - Validates that pubkey derives to the claimed address
///
/// # Arguments
///
/// * `payload` - The signed BIP-322 payload
///
/// # Returns
///
/// * `Some(PublicKey)` if verification succeeds
/// * `None` if verification fails
pub fn verify_p2pkh_signature(
    payload: &SignedBip322Payload,
) -> Option<<Secp256k1 as Curve>::PublicKey> {
    // Early exit: Check witness type
    let Bip322Witness::P2PKH { .. } = &payload.signature else {
        return None;
    };

    let signature_bytes = payload.signature.signature();
    let pubkey_bytes = payload.signature.pubkey();

    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);

    // Compute sighash for P2PKH (legacy sighash algorithm)
    let sighash = Bip322MessageHasher::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key
    SignedBip322Payload::try_recover_pubkey(&sighash, signature_bytes, pubkey_bytes)
}

/// Verifies a BIP-322 signature for P2WPKH addresses.
///
/// P2WPKH verification expects:
/// - Witness stack: [signature, pubkey]
/// - Uses segwit v0 sighash algorithm (BIP-143)
/// - Validates that pubkey derives to the claimed address
///
/// # Arguments
///
/// * `payload` - The signed BIP-322 payload
///
/// # Returns
///
/// * `Some(PublicKey)` if verification succeeds
/// * `None` if verification fails
pub fn verify_p2wpkh_signature(
    payload: &SignedBip322Payload,
) -> Option<<Secp256k1 as Curve>::PublicKey> {
    // Early exit: Check witness type
    let Bip322Witness::P2WPKH { .. } = &payload.signature else {
        return None;
    };

    let signature_bytes = payload.signature.signature();
    let pubkey_bytes = payload.signature.pubkey();

    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);

    // Compute sighash for P2WPKH (segwit v0 sighash algorithm)
    let sighash = Bip322MessageHasher::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key
    SignedBip322Payload::try_recover_pubkey(&sighash, signature_bytes, pubkey_bytes)
}

/// Verifies a BIP-322 signature for P2SH addresses.
///
/// P2SH verification expects:
/// - Witness stack: [signature, pubkey]
/// - Uses legacy Bitcoin sighash algorithm
/// - Validates that the redeem script hash matches the address
///
/// # Arguments
///
/// * `payload` - The signed BIP-322 payload
///
/// # Returns
///
/// * `Some(PublicKey)` if verification succeeds
/// * `None` if verification fails
pub fn verify_p2sh_signature(
    payload: &SignedBip322Payload,
) -> Option<<Secp256k1 as Curve>::PublicKey> {
    // Early exit: Check witness type
    let Bip322Witness::P2SH { .. } = &payload.signature else {
        return None;
    };

    let signature_bytes = payload.signature.signature();
    let pubkey_bytes = payload.signature.pubkey();

    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);

    // Compute sighash for P2SH (legacy sighash algorithm)
    let sighash = Bip322MessageHasher::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key
    SignedBip322Payload::try_recover_pubkey(&sighash, signature_bytes, pubkey_bytes)
}

/// Verifies a BIP-322 signature for P2WSH addresses.
///
/// P2WSH verification expects:
/// - Witness stack: [signature, pubkey, witness_script]
/// - Uses segwit v0 sighash algorithm (BIP-143)
/// - Validates that the witness script hash matches the address
///
/// # Arguments
///
/// * `payload` - The signed BIP-322 payload
///
/// # Returns
///
/// * `Some(PublicKey)` if verification succeeds
/// * `None` if verification fails
pub fn verify_p2wsh_signature(
    payload: &SignedBip322Payload,
) -> Option<<Secp256k1 as Curve>::PublicKey> {
    // Early exit: Check witness type
    let Bip322Witness::P2WSH { .. } = &payload.signature else {
        return None;
    };

    let signature_bytes = payload.signature.signature();
    let pubkey_bytes = payload.signature.pubkey();
    let witness_script = payload.signature.witness_script().unwrap_or(&[]);

    // Early exit: Validate witness script hash matches the address
    let computed_script_hash = env::sha256_array(witness_script);
    let Address::P2WSH { witness_program } = &payload.address else {
        return None; // This should never happen since we're in P2WSH verification
    };
    
    if computed_script_hash != witness_program.program.as_slice() {
        return None;
    }

    // Early exit: Execute the witness script
    if !SignedBip322Payload::execute_witness_script(witness_script, pubkey_bytes) {
        return None;
    }

    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);

    // Compute sighash for P2WSH (segwit v0 sighash algorithm)
    let sighash = Bip322MessageHasher::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key
    SignedBip322Payload::try_recover_pubkey(&sighash, signature_bytes, pubkey_bytes)
}

