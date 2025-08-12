//! BIP-322 message hashing logic
//!
//! This module contains the hashing algorithms used in BIP-322 signature verification.
//! It includes both the BIP-322 tagged hash for messages and the sighash computation
//! methods for different address types.

use crate::bitcoin_minimal::{Address, EcdsaSighashType, ScriptBuf, Transaction, OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160};
use defuse_near_utils::digest::{DoubleSha256, Sha256, TaggedDigest};
use digest::Digest;

/// BIP-322 message hashing utilities
pub struct Bip322MessageHasher;

impl Bip322MessageHasher {
    /// Computes the BIP-322 tagged message hash using BIP-340 tagged digest implementation.
    ///
    /// BIP-322 uses a "tagged hash" approach identical to BIP-340 (Schnorr signatures).
    /// This prevents signature reuse across different contexts by domain-separating
    /// the hash computation.
    ///
    /// The tagged hash algorithm:
    /// 1. Compute `tag_hash = SHA256("BIP0322-signed-message")`
    /// 2. Compute `message_hash = SHA256(tag_hash || tag_hash || message)`
    ///
    /// This implementation uses the BIP-340 `Bip340TaggedDigest` trait with our
    /// NEAR SDK compatible SHA-256 implementation for optimal gas efficiency.
    ///
    /// # Arguments
    ///
    /// * `message` - The message string to hash
    ///
    /// # Returns
    ///
    /// A 32-byte hash that represents the BIP-322 tagged hash of the message.
    pub fn compute_bip322_message_hash(message: &str) -> [u8; 32] {
        // Use BIP-340's tagged digest implementation with NEAR SDK SHA-256
        Sha256::tagged(b"BIP0322-signed-message")
            .chain_update(message.as_bytes())
            .finalize()
            .into()
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
                Self::compute_segwit_v0_sighash(to_spend, to_sign, address)
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

        DoubleSha256::digest(&buf).into()
    }

    /// Compute segwit v0 sighash for P2WPKH and P2WSH addresses.
    ///
    /// This implements the BIP-143 sighash algorithm introduced with segwit.
    /// It fixes several issues with the legacy algorithm and includes the
    /// amount being spent in the signature hash.
    /// Note: For P2WPKH, scriptCode must be the P2PKH template, not the witness program.
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
        address: &Address,
    ) -> near_sdk::CryptoHash {
        // Build the correct scriptCode depending on address type
        let script_code = match address {
            Address::P2WPKH { witness_program } => {
                // Expect version 0 and 20-byte program
                assert!(
                    witness_program.version == 0 && witness_program.program.len() == 20,
                    "P2WPKH witness program must be v0 with 20-byte hash"
                );

                // OP_DUP OP_HASH160 <20> <pubkey-hash> OP_EQUALVERIFY OP_CHECKSIG
                let mut sc = Vec::with_capacity(25);
                sc.push(OP_DUP);
                sc.push(OP_HASH160);
                sc.push(20);
                sc.extend_from_slice(&witness_program.program);
                sc.push(OP_EQUALVERIFY);
                sc.push(OP_CHECKSIG);
                ScriptBuf::from_bytes(sc)
            }
            Address::P2WSH { .. } => {
                // For P2WSH, the scriptCode must be the witness script itself.
                // It is not derivable from the address; you'll need the script provided.
                // If you don't support general P2WSH here, you can return a hash that will
                // never verify, or panic with a clear message.
                panic!("compute_segwit_v0_sighash: P2WSH requires the witness script (not derivable from address)")
            }
            // Should not reach here; function only called for segwit types
            _ => unreachable!("compute_segwit_v0_sighash called with non-segwit address"),
        };

        let amount = to_spend
            .output
            .first()
            .expect("to_spend should have output")
            .value;

        let mut buf = Vec::with_capacity(200);
        to_sign
            .encode_segwit_v0(&mut buf, 0, &script_code, amount, EcdsaSighashType::All)
            .expect("Segwit v0 sighash encoding should succeed");

        DoubleSha256::digest(&buf).into()
    }
}
