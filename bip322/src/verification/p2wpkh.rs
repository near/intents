//! P2WPKH (Pay-to-Witness-Public-Key-Hash) BIP-322 verification logic
//!
//! P2WPKH addresses use the segwit v0 sighash algorithm (BIP-143) for signature verification.
//! The witness stack format is [signature, pubkey].

use crate::SignedBip322Payload;
use defuse_crypto::{Curve, Secp256k1};
use near_sdk::CryptoHash;

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
    // For P2WPKH, witness should contain [signature, pubkey]
    if payload.signature.len() < 2 {
        return None;
    }

    let signature_bytes = payload.signature.nth(0)?;
    let pubkey_bytes = payload.signature.nth(1)?;

    // Create BIP-322 transactions
    let to_spend = payload.create_to_spend();
    let to_sign = SignedBip322Payload::create_to_sign(&to_spend);

    // Compute sighash for P2WPKH (segwit v0 sighash algorithm)
    let sighash = SignedBip322Payload::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    );

    // Try to recover public key
    SignedBip322Payload::try_recover_pubkey(&sighash, signature_bytes, pubkey_bytes)
}

/// Computes the BIP-322 message hash for P2WPKH addresses.
///
/// P2WPKH uses the segwit v0 sighash algorithm (BIP-143) for message hash computation.
///
/// # Arguments
///
/// * `payload` - The BIP-322 payload containing the message and address
///
/// # Returns
///
/// The 32-byte message hash for P2WPKH signature verification
pub fn compute_p2wpkh_message_hash(payload: &SignedBip322Payload) -> CryptoHash {
    // Step 1: Create the "to_spend" transaction
    let to_spend = payload.create_to_spend();

    // Step 2: Create the "to_sign" transaction
    let to_sign = SignedBip322Payload::create_to_sign(&to_spend);

    // Step 3: Compute signature hash using segwit v0 algorithm
    SignedBip322Payload::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    )
}
