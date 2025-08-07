//! BIP-322 signature verification logic
//!
//! This module contains unified verification logic for all Bitcoin address types.
//! Each verification function uses early exit patterns for cleaner, more readable code.

use crate::bitcoin_minimal::Address;
use crate::hashing::Bip322MessageHasher;
use crate::transaction::Bip322TransactionBuilder;
use crate::SignedBip322Payload;
use defuse_crypto::{Curve, Secp256k1};
use near_sdk::env;

/// Verifies a BIP-322 signature for P2PKH addresses.
///
/// P2PKH verification expects:
/// - 65-byte compact signature format
/// - Uses legacy Bitcoin sighash algorithm
/// - Recovers pubkey from signature and validates against address
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
    // Ensure this is a P2PKH address
    let Address::P2PKH { pubkey_hash } = &payload.address else {
        return None;
    };

    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);

    // Compute sighash for P2PKH (legacy sighash algorithm)
    let sighash = Bip322MessageHasher::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key from signature
    let recovered_pubkey = SignedBip322Payload::try_recover_pubkey(&sighash, &payload.signature)?;
    
    // Verify that the recovered pubkey matches the P2PKH address
    let computed_pubkey_hash = crate::bitcoin_minimal::hash160(&recovered_pubkey);
    if computed_pubkey_hash == *pubkey_hash {
        Some(recovered_pubkey)
    } else {
        None
    }
}

/// Verifies a BIP-322 signature for P2WPKH addresses.
///
/// P2WPKH verification expects:
/// - 65-byte compact signature format
/// - Uses segwit v0 sighash algorithm (BIP-143)
/// - Recovers pubkey from signature and validates against address
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
    // Ensure this is a P2WPKH address
    let Address::P2WPKH { witness_program } = &payload.address else {
        return None;
    };

    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);

    // Compute sighash for P2WPKH (segwit v0 sighash algorithm)
    let sighash = Bip322MessageHasher::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key from signature
    let recovered_pubkey = SignedBip322Payload::try_recover_pubkey(&sighash, &payload.signature)?;
    
    // Verify that the recovered pubkey matches the P2WPKH address
    let computed_pubkey_hash = crate::bitcoin_minimal::hash160(&recovered_pubkey);
    if computed_pubkey_hash == witness_program.program.as_slice() {
        Some(recovered_pubkey)
    } else {
        None
    }
}

/// Verifies a BIP-322 signature for P2SH addresses.
///
/// P2SH verification expects:
/// - 65-byte compact signature format  
/// - Uses legacy Bitcoin sighash algorithm
/// - Limited support: only P2PKH-style redeem scripts are supported
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
    // Ensure this is a P2SH address
    let Address::P2SH { script_hash } = &payload.address else {
        return None;
    };

    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);

    // Compute sighash for P2SH (legacy sighash algorithm)
    let sighash = Bip322MessageHasher::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key from signature
    let recovered_pubkey = SignedBip322Payload::try_recover_pubkey(&sighash, &payload.signature)?;
    
    // For simplified P2SH support, assume P2PKH-style redeem script and verify
    // that the recovered pubkey generates a script hash matching the address
    let pubkey_hash = crate::bitcoin_minimal::hash160(&recovered_pubkey);
    
    // Create P2PKH-style redeem script: OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG
    let mut redeem_script = Vec::with_capacity(25);
    redeem_script.push(0x76); // OP_DUP
    redeem_script.push(0xa9); // OP_HASH160  
    redeem_script.push(0x14); // Push 20 bytes
    redeem_script.extend_from_slice(&pubkey_hash);
    redeem_script.push(0x88); // OP_EQUALVERIFY
    redeem_script.push(0xac); // OP_CHECKSIG
    
    // Hash the redeem script and compare with address script hash
    let computed_script_hash = crate::bitcoin_minimal::hash160(&redeem_script);
    if computed_script_hash == *script_hash {
        Some(recovered_pubkey)
    } else {
        None
    }
}

/// Verifies a BIP-322 signature for P2WSH addresses.
///
/// P2WSH verification expects:
/// - 65-byte compact signature format
/// - Uses segwit v0 sighash algorithm (BIP-143)  
/// - Limited support: only P2PKH-style witness scripts are supported
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
    // Ensure this is a P2WSH address
    let Address::P2WSH { witness_program } = &payload.address else {
        return None;
    };

    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);

    // Compute sighash for P2WSH (segwit v0 sighash algorithm)
    let sighash = Bip322MessageHasher::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key from signature
    let recovered_pubkey = SignedBip322Payload::try_recover_pubkey(&sighash, &payload.signature)?;
    
    // For simplified P2WSH support, assume P2PKH-style witness script and verify
    // that the recovered pubkey generates a witness script hash matching the address
    let pubkey_hash = crate::bitcoin_minimal::hash160(&recovered_pubkey);
    
    // Create P2PKH-style witness script: OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG
    let mut witness_script = Vec::with_capacity(25);
    witness_script.push(0x76); // OP_DUP
    witness_script.push(0xa9); // OP_HASH160
    witness_script.push(0x14); // Push 20 bytes
    witness_script.extend_from_slice(&pubkey_hash);
    witness_script.push(0x88); // OP_EQUALVERIFY
    witness_script.push(0xac); // OP_CHECKSIG
    
    // Hash the witness script with SHA256 (not hash160) and compare with address
    let computed_script_hash = env::sha256_array(&witness_script);
    if computed_script_hash == witness_program.program.as_slice() {
        Some(recovered_pubkey)
    } else {
        None
    }
}

