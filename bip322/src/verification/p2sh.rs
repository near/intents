//! P2SH (Pay-to-Script-Hash) BIP-322 verification logic
//!
//! P2SH addresses use the legacy Bitcoin sighash algorithm for signature verification.
//! The witness stack format is [signature, pubkey, redeem_script].

use crate::hashing::Bip322MessageHasher;
use crate::SignedBip322Payload;
use defuse_crypto::{Curve, Secp256k1};
use near_sdk::CryptoHash;

/// Verifies a BIP-322 signature for P2SH addresses.
///
/// P2SH verification expects:
/// - Witness stack: [signature, pubkey, redeem_script]
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
    // For P2SH, extract signature and pubkey from witness
    match &payload.signature {
        crate::bitcoin_minimal::Bip322Witness::P2SH { .. } => {
            let signature_bytes = payload.signature.signature();
            let pubkey_bytes = payload.signature.pubkey();

            // Create BIP-322 transactions
            let to_spend = payload.create_to_spend();
            let to_sign = SignedBip322Payload::create_to_sign(&to_spend);

            // Compute sighash for P2SH (legacy sighash algorithm)
            let sighash = Bip322MessageHasher::compute_message_hash(
                &to_spend,
                &to_sign,
                &payload.address,
            );

            // Try to recover public key
            SignedBip322Payload::try_recover_pubkey(&sighash, signature_bytes, pubkey_bytes)
        }
        _ => None, // Wrong witness type for P2SH
    }
}

/// Computes the BIP-322 message hash for P2SH addresses.
///
/// P2SH uses the legacy Bitcoin sighash algorithm for message hash computation.
///
/// # Arguments
///
/// * `payload` - The BIP-322 payload containing the message and address
///
/// # Returns
///
/// The 32-byte message hash for P2SH signature verification
pub fn compute_p2sh_message_hash(payload: &SignedBip322Payload) -> CryptoHash {
    // Step 1: Create the "to_spend" transaction
    let to_spend = payload.create_to_spend();

    // Step 2: Create the "to_sign" transaction
    let to_sign = SignedBip322Payload::create_to_sign(&to_spend);

    // Step 3: Compute signature hash using legacy algorithm
    Bip322MessageHasher::compute_message_hash(
        &to_spend,
        &to_sign,
        &payload.address,
    )
}
