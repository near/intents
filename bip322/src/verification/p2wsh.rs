//! P2WSH (Pay-to-Witness-Script-Hash) BIP-322 verification logic
//!
//! P2WSH addresses use the segwit v0 sighash algorithm (BIP-143) for signature verification.
//! The witness stack format is [signature, pubkey, witness_script].

use crate::SignedBip322Payload;
use crate::bitcoin_minimal::Address;
use defuse_crypto::{Curve, Secp256k1};
use near_sdk::{CryptoHash, env};

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
    // For P2WSH, the witness should contain [signature, pubkey, witness_script]
    if payload.signature.len() < 3 {
        return None;
    }

    let signature_bytes = payload.signature.nth(0)?;
    let pubkey_bytes = payload.signature.nth(1)?;
    let witness_script = payload.signature.nth(2)?;

    // Validate witness script hash matches the address
    let computed_script_hash = env::sha256_array(witness_script);
    if let Address::P2WSH { witness_program } = &payload.address {
        if computed_script_hash != witness_program.program.as_slice() {
            return None;
        }
    } else {
        // This should never happen since we're in P2WSH verification
        return None;
    }

    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = SignedBip322Payload::create_to_sign(&to_spend);

    // Compute sighash for P2WSH (segwit v0 sighash algorithm)
    let sighash = SignedBip322Payload::compute_message_hash_for_address(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key
    SignedBip322Payload::try_recover_pubkey(&sighash, signature_bytes, pubkey_bytes)
}

/// Computes the BIP-322 message hash for P2WSH addresses.
///
/// P2WSH uses the segwit v0 sighash algorithm (BIP-143) for message hash computation.
///
/// # Arguments
///
/// * `payload` - The BIP-322 payload containing the message and address
///
/// # Returns
///
/// The 32-byte message hash for P2WSH signature verification
pub fn compute_p2wsh_message_hash(payload: &SignedBip322Payload) -> CryptoHash {
    // Step 1: Create the "to_spend" transaction
    let to_spend = payload.create_to_spend();

    // Step 2: Create the "to_sign" transaction
    let to_sign = SignedBip322Payload::create_to_sign(&to_spend);

    // Step 3: Compute signature hash using segwit v0 algorithm
    SignedBip322Payload::compute_message_hash_for_address(
        &to_spend,
        &to_sign,
        &payload.address,
    )
}
