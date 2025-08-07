//! BIP-322 message hashing logic
//!
//! This module contains the hashing algorithms used in BIP-322 signature verification.
//! It includes both the BIP-322 tagged hash for messages and the sighash computation
//! methods for different address types.

use crate::bitcoin_minimal::{Address, EcdsaSighashType, NearDoubleSha256, Transaction};
use digest::Digest;
use near_sdk::env;

/// BIP-322 message hashing utilities
pub struct Bip322MessageHasher;

impl Bip322MessageHasher {
    /// Computes the BIP-322 tagged message hash using NEAR SDK cryptographic functions.
    ///
    /// BIP-322 uses a "tagged hash" approach similar to BIP-340 (Schnorr signatures).
    /// This prevents signature reuse across different contexts by domain-separating
    /// the hash computation.
    ///
    /// The tagged hash algorithm:
    /// 1. Compute `tag_hash = SHA256("BIP0322-signed-message")`
    /// 2. Compute `message_hash = SHA256(tag_hash || tag_hash || message)`
    ///
    /// This double-inclusion of the tag hash ensures domain separation while
    /// maintaining compatibility with existing SHA256 implementations.
    ///
    /// # Arguments
    ///
    /// * `message` - The message string to hash
    ///
    /// # Returns
    ///
    /// A 32-byte hash that represents the BIP-322 tagged hash of the message.
    pub fn compute_bip322_message_hash(message: &str) -> [u8; 32] {
        // The BIP-322 tag string - this creates domain separation
        let tag = b"BIP0322-signed-message";

        // Hash the tag itself using NEAR SDK
        let tag_hash = env::sha256_array(tag);

        // Create the tagged hash: SHA256(tag_hash || tag_hash || message)
        // The double tag_hash inclusion is part of the BIP-340 tagged hash specification
        let mut input = Vec::with_capacity(tag_hash.len() * 2 + message.len());
        input.extend_from_slice(&tag_hash); // First tag hash
        input.extend_from_slice(&tag_hash); // Second tag hash (domain separation)
        input.extend_from_slice(message.as_bytes()); // The actual message

        // Final hash computation using NEAR SDK
        env::sha256_array(&input)
    }

    /// Compute the message hash using the appropriate sighash algorithm based on address type.
    ///
    /// Bitcoin uses different sighash algorithms:
    /// - Legacy sighash: For P2PKH and P2SH addresses (pre-segwit)
    /// - Segwit v0 sighash: For P2WPKH and P2WSH addresses (BIP-143)
    ///
    /// # Arguments
    ///
    /// * `to_spend` - The "to_spend" BIP-322 transaction
    /// * `to_sign` - The "to_sign" BIP-322 transaction
    /// * `address` - The address type determines which sighash algorithm to use
    ///
    /// # Returns
    ///
    /// The computed sighash as a 32-byte array
    pub fn compute_message_hash(
        to_spend: &Transaction,
        to_sign: &Transaction,
        address: &Address,
    ) -> near_sdk::CryptoHash {
        match address {
            Address::P2PKH { .. } | Address::P2SH { .. } => {
                Self::compute_legacy_sighash(to_spend, to_sign)
            }
            Address::P2WPKH { .. } | Address::P2WSH { .. } => {
                Self::compute_segwit_v0_sighash(to_spend, to_sign)
            }
        }
    }

    /// Compute legacy sighash for P2PKH and P2SH addresses.
    ///
    /// This implements the original Bitcoin sighash algorithm used before segwit.
    /// It's simpler than the segwit version but has some known vulnerabilities
    /// (like quadratic scaling) that segwit addresses.
    ///
    /// # Arguments
    ///
    /// * `to_spend` - The "to_spend" BIP-322 transaction
    /// * `to_sign` - The "to_sign" BIP-322 transaction
    ///
    /// # Returns
    ///
    /// The legacy sighash as a 32-byte NEAR CryptoHash
    pub fn compute_legacy_sighash(
        to_spend: &Transaction,
        to_sign: &Transaction,
    ) -> near_sdk::CryptoHash {
        let script_code = &to_spend
            .output
            .first()
            .expect("to_spend should have output")
            .script_pubkey;

        // Legacy sighash preimage is typically ~200-400 bytes
        let mut buf = Vec::with_capacity(400);
        to_sign
            .encode_legacy(&mut buf, 0, script_code, EcdsaSighashType::All)
            .expect("Legacy sighash encoding should succeed");

        NearDoubleSha256::digest(&buf).into()
    }

    /// Compute segwit v0 sighash for P2WPKH and P2WSH addresses.
    ///
    /// This implements the BIP-143 sighash algorithm introduced with segwit.
    /// It fixes several issues with the legacy algorithm and includes the
    /// amount being spent in the signature hash.
    ///
    /// # Arguments
    ///
    /// * `to_spend` - The "to_spend" BIP-322 transaction
    /// * `to_sign` - The "to_sign" BIP-322 transaction
    ///
    /// # Returns
    ///
    /// The segwit v0 sighash as a 32-byte NEAR CryptoHash
    pub fn compute_segwit_v0_sighash(
        to_spend: &Transaction,
        to_sign: &Transaction,
    ) -> near_sdk::CryptoHash {
        let script_code = &to_spend
            .output
            .first()
            .expect("to_spend should have output")
            .script_pubkey;

        // BIP-143 sighash preimage has fixed structure: ~200 bytes
        let mut buf = Vec::with_capacity(200);
        to_sign
            .encode_segwit_v0(
                &mut buf,
                0,
                script_code,
                to_spend
                    .output
                    .first()
                    .expect("to_spend should have output")
                    .value,
                EcdsaSighashType::All,
            )
            .expect("Segwit v0 sighash encoding should succeed");

        NearDoubleSha256::digest(&buf).into()
    }
}