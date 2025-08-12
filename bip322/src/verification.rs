//! BIP-322 signature verification logic
//!
//! This module contains unified verification logic for all Bitcoin address types.
//! Uses a common verification pattern with address-specific validation.

use crate::SignedBip322Payload;
use crate::bitcoin_minimal::{Address, OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160};
use crate::hashing::Bip322MessageHasher;
use crate::transaction::create_to_sign;
use defuse_crypto::{Curve, Secp256k1};
use near_sdk::env;

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
        response.push(crate::bitcoin_minimal::hash160(&compressed));

        compressed.as_mut_slice()[0] = 0x03;
        response.push(crate::bitcoin_minimal::hash160(&compressed));

        return response
    }

    vec![crate::bitcoin_minimal::hash160(raw_pubkey)]
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

/// Common BIP-322 verification logic that recovers the public key.
///
/// This function implements the standard BIP-322 verification process:
/// 1. Creates BIP-322 transactions (to_spend, to_sign)
/// 2. Computes message hash using appropriate algorithm for address type
/// 3. Recovers public key from compact signature
///
/// # Arguments
///
/// * `payload` - The signed BIP-322 payload
///
/// # Returns
///
/// * `Some(PublicKey)` if public key recovery succeeds
/// * `None` if recovery fails
fn verify_bip322_common(payload: &SignedBip322Payload) -> Option<<Secp256k1 as Curve>::PublicKey> {
    let to_spend = payload.create_to_spend();
    let to_sign = create_to_sign(&to_spend);

    let sighash = Bip322MessageHasher::compute_message_hash(&to_spend, &to_sign, &payload.address);

    SignedBip322Payload::try_recover_pubkey(&sighash, &payload.signature)
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

    let recovered_pubkey = verify_bip322_common(payload)?;

    // Validate that recovered pubkey matches the P2PKH address
    // P2PKH can use either compressed or uncompressed, try both

    // Try uncompressed first
    let uncompressed_hash = hash160_pubkey(&recovered_pubkey, false);
    if uncompressed_hash[0] == *pubkey_hash {
        return Some(recovered_pubkey)
    }

    // Try compressed next, two possibilities
    let compressed_hash = hash160_pubkey(&recovered_pubkey, true);
    if compressed_hash[0] == *pubkey_hash {
        return Some(recovered_pubkey);
    }

    if compressed_hash[1] == *pubkey_hash {
        return Some(recovered_pubkey);
    }

    None
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

    let recovered_pubkey = verify_bip322_common(payload)?;

    // Validate that recovered pubkey matches the P2WPKH address
    // P2WPKH addresses always use compressed public keys, so two possibilities,
    // depending on the y coordinate parity
    let computed_pubkey_hash = hash160_pubkey(&recovered_pubkey, true);

    if computed_pubkey_hash[0] == witness_program.program.as_slice() {
        return Some(recovered_pubkey);
    }

    if computed_pubkey_hash[1] == witness_program.program.as_slice() {
        return Some(recovered_pubkey);
    }

    None
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

    let recovered_pubkey = verify_bip322_common(payload)?;

    // Create P2PKH-style redeem script from recovered pubkey
    // There is no fixed rule, public keys can be compressed or uncompressed,
    // so we have to try both.

    // Try uncompressed first
    let pubkey_hash = hash160_pubkey(&recovered_pubkey, false);
    let redeem_script = build_script(&pubkey_hash[0]);
    let computed_script_hash = crate::bitcoin_minimal::hash160(&redeem_script);

    if computed_script_hash == *script_hash {
        return Some(recovered_pubkey);
    }

    // Try compressed next, two possibilities
    let pubkey_hash = hash160_pubkey(&recovered_pubkey, true);

    let redeem_script = build_script(&pubkey_hash[0]);
    let computed_script_hash = crate::bitcoin_minimal::hash160(&redeem_script);
    if computed_script_hash == *script_hash {
        return Some(recovered_pubkey);
    }

    let redeem_script = build_script(&pubkey_hash[1]);
    let computed_script_hash = crate::bitcoin_minimal::hash160(&redeem_script);
    if computed_script_hash == *script_hash {
        return Some(recovered_pubkey);
    }

    // Both failed, return None
    None
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

    let recovered_pubkey = verify_bip322_common(payload)?;

    // Create P2PKH-style witness script from recovered pubkey
    // Try uncompressed first
    let pubkey_hash = hash160_pubkey(&recovered_pubkey, false);

    let witness_script = build_script(&pubkey_hash[0]);
    let computed_script_hash = env::sha256_array(&witness_script);

    if computed_script_hash == witness_program.program.as_slice() {
        return Some(recovered_pubkey);
    }

    // Try compressed next
    let pubkey_hash = hash160_pubkey(&recovered_pubkey, true);

    let witness_script = build_script(&pubkey_hash[0]);
    let computed_script_hash = env::sha256_array(&witness_script);
    if computed_script_hash == witness_program.program.as_slice() {
        return Some(recovered_pubkey);
    }

    let witness_script = build_script(&pubkey_hash[1]);
    let computed_script_hash = env::sha256_array(&witness_script);
    if computed_script_hash == witness_program.program.as_slice() {
        return Some(recovered_pubkey);
    }

    // Both failed, return None
    None
}
