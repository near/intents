//! BIP-322 signature verification logic
//!
//! This module contains unified verification logic for all Bitcoin address types.
//! Uses a common verification pattern with address-specific validation.

use crate::bitcoin_minimal::Address;
use crate::hashing::Bip322MessageHasher;
use crate::transaction::Bip322TransactionBuilder;
use crate::SignedBip322Payload;
use defuse_crypto::{Curve, Secp256k1};
use near_sdk::env;

/// Common BIP-322 verification logic.
///
/// This function implements the standard BIP-322 verification process:
/// 1. Creates BIP-322 transactions (to_spend, to_sign)
/// 2. Computes message hash using appropriate algorithm for address type
/// 3. Recovers public key from compact signature
/// 4. Calls address-specific validation function
///
/// # Arguments
/// 
/// * `payload` - The signed BIP-322 payload
/// * `validate_address` - Function to validate recovered pubkey against specific address
///
/// # Returns
///
/// * `Some(PublicKey)` if verification succeeds
/// * `None` if verification fails
fn verify_bip322_common<F>(
    payload: &SignedBip322Payload,
    validate_address: F,
) -> Option<<Secp256k1 as Curve>::PublicKey>
where
    F: FnOnce(&[u8; 64]) -> bool,
{
    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = Bip322TransactionBuilder::create_to_sign(&to_spend);

    // Compute sighash using appropriate algorithm for address type
    let sighash = Bip322MessageHasher::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key from signature
    let recovered_pubkey = SignedBip322Payload::try_recover_pubkey(&sighash, &payload.signature)?;
    
    // Validate recovered pubkey against address using provided function
    if validate_address(&recovered_pubkey) {
        Some(recovered_pubkey)
    } else {
        None
    }
}

/// Verifies a BIP-322 signature for P2PKH addresses.
///
/// P2PKH verification recovers the public key from the signature and validates
/// that its hash160 matches the pubkey_hash in the P2PKH address.
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

    // Use common verification with P2PKH-specific validation
    verify_bip322_common(payload, |recovered_pubkey| {
        let computed_pubkey_hash = crate::bitcoin_minimal::hash160(recovered_pubkey);
        computed_pubkey_hash == *pubkey_hash
    })
}

/// Verifies a BIP-322 signature for P2WPKH addresses.
///
/// P2WPKH verification recovers the public key from the signature and validates
/// that its hash160 matches the witness program (pubkey hash) in the P2WPKH address.
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

    // Use common verification with P2WPKH-specific validation
    verify_bip322_common(payload, |recovered_pubkey| {
        let computed_pubkey_hash = crate::bitcoin_minimal::hash160(recovered_pubkey);
        computed_pubkey_hash == witness_program.program.as_slice()
    })
}

/// Verifies a BIP-322 signature for P2SH addresses.
///
/// P2SH verification creates a P2PKH-style redeem script from the recovered
/// public key and validates that its hash160 matches the script_hash in the P2SH address.
/// This is a simplified implementation that only supports P2PKH-style redeem scripts.
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

    // Use common verification with P2SH-specific validation
    verify_bip322_common(payload, |recovered_pubkey| {
        // Create P2PKH-style redeem script from recovered pubkey
        let pubkey_hash = crate::bitcoin_minimal::hash160(recovered_pubkey);
        let mut redeem_script = Vec::with_capacity(25);
        redeem_script.push(0x76); // OP_DUP
        redeem_script.push(0xa9); // OP_HASH160  
        redeem_script.push(0x14); // Push 20 bytes
        redeem_script.extend_from_slice(&pubkey_hash);
        redeem_script.push(0x88); // OP_EQUALVERIFY
        redeem_script.push(0xac); // OP_CHECKSIG
        
        // Hash the redeem script and compare with address script hash
        let computed_script_hash = crate::bitcoin_minimal::hash160(&redeem_script);
        computed_script_hash == *script_hash
    })
}

/// Verifies a BIP-322 signature for P2WSH addresses.
///
/// P2WSH verification creates a P2PKH-style witness script from the recovered
/// public key and validates that its SHA256 hash matches the witness program in the P2WSH address.
/// This is a simplified implementation that only supports P2PKH-style witness scripts.
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

    // Use common verification with P2WSH-specific validation
    verify_bip322_common(payload, |recovered_pubkey| {
        // Create P2PKH-style witness script from recovered pubkey
        let pubkey_hash = crate::bitcoin_minimal::hash160(recovered_pubkey);
        let mut witness_script = Vec::with_capacity(25);
        witness_script.push(0x76); // OP_DUP
        witness_script.push(0xa9); // OP_HASH160
        witness_script.push(0x14); // Push 20 bytes
        witness_script.extend_from_slice(&pubkey_hash);
        witness_script.push(0x88); // OP_EQUALVERIFY
        witness_script.push(0xac); // OP_CHECKSIG
        
        // Hash witness script with SHA256 (not hash160) and compare with address
        let computed_script_hash = env::sha256_array(&witness_script);
        computed_script_hash == witness_program.program.as_slice()
    })
}

