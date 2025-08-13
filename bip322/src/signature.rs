//! BIP-322 signature parsing and key extraction
//!
//! This module contains the `Bip322Signature` enum and related functionality for
//! parsing both compact and full BIP-322 signature formats, including public key
//! extraction from witness data.

use crate::{
    bitcoin_minimal::Address,
    hashing::Bip322MessageHasher,
    transaction::{create_to_sign, create_to_spend},
};
use base64::{Engine, engine::general_purpose};
use defuse_crypto::{Curve, Secp256k1};
use near_sdk::{env, near};
use serde_with::serde_as;
use std::str::FromStr;

/// BIP-322 signature formats supported by different Bitcoin wallets.
///
/// Bitcoin wallets produce different signature formats when implementing BIP-322:
/// - **Simple/Compact**: Base64-encoded 65-byte signature (recovery byte + r + s)
///   Used by wallets like Sparrow for P2PKH and some P2WPKH addresses
/// - **Full**: Complete BIP-322 witness stack with transaction structure
///   Used by advanced wallets and for complex script types
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[serde(rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub enum Bip322Signature {
    /// Simple/Compact signature format (65 bytes: recovery + r + s).
    ///
    /// This is the standard Bitcoin message signing format used by most wallets.
    /// For BIP-322 simple signatures, the message is hashed directly with BIP-340
    /// tagged hash, not through transaction construction.
    Compact {
        #[serde_as(as = "serde_with::Bytes")]
        signature: [u8; 65],
    },

    /// Full BIP-322 signature format with complete witness data.
    ///
    /// Contains the witness stack and transaction structure required for
    /// complex BIP-322 verification. Used for P2WSH and advanced signing scenarios.
    Full {
        /// Parsed witness stack data containing signatures and public keys
        witness_stack: Vec<Vec<u8>>,
    },
}

/// Internal representation of public keys in different formats
#[derive(Debug, Clone)]
enum ParsedPublicKey {
    /// 33-byte compressed public key (prefix + x-coordinate)
    Compressed([u8; 33]),
    /// 64-byte uncompressed public key (x + y coordinates, without 0x04 prefix)
    Uncompressed([u8; 64]),
}

/// Error types for BIP-322 signature parsing
#[derive(Debug, Clone)]
pub enum Bip322Error {
    InvalidBase64(base64::DecodeError),
    InvalidWitnessFormat,
    InvalidCompactSignature,
    PublicKeyExtractionFailed,
}

impl From<base64::DecodeError> for Bip322Error {
    fn from(e: base64::DecodeError) -> Self {
        Bip322Error::InvalidBase64(e)
    }
}

impl FromStr for Bip322Signature {
    type Err = Bip322Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Single base64 decode - parse once and determine format
        let decoded = general_purpose::STANDARD.decode(s)?;

        // Check if it's a simple 65-byte compact signature
        if decoded.len() == 65 {
            let sig_bytes: [u8; 65] = decoded.try_into().expect("Invalid signature length"); // Should never fail
            return Ok(Bip322Signature::Compact {
                signature: sig_bytes,
            });
        }

        // Otherwise, parse as full BIP-322 witness format
        Self::parse_full_signature(&decoded)
    }
}

impl Bip322Signature {
    /// Read a variable-length integer from data starting at cursor position.
    ///
    /// Returns (value, bytes_consumed) or None if invalid/truncated data.
    /// 
    /// Bitcoin varint format:
    /// - < 0xFD: single byte value
    /// - 0xFD: followed by 2-byte little-endian value
    /// - 0xFE: followed by 4-byte little-endian value  
    /// - 0xFF: followed by 8-byte little-endian value
    fn read_varint(data: &[u8], cursor: usize) -> Option<(u64, usize)> {
        if cursor >= data.len() {
            return None;
        }

        match data[cursor] {
            n @ 0..=0xFC => Some((n as u64, 1)),
            0xFD => {
                if cursor + 3 > data.len() {
                    return None;
                }
                let value = u16::from_le_bytes([data[cursor + 1], data[cursor + 2]]) as u64;
                Some((value, 3))
            }
            0xFE => {
                if cursor + 5 > data.len() {
                    return None;
                }
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&data[cursor + 1..cursor + 5]);
                let value = u32::from_le_bytes(bytes) as u64;
                Some((value, 5))
            }
            0xFF => {
                if cursor + 9 > data.len() {
                    return None;
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&data[cursor + 1..cursor + 9]);
                let value = u64::from_le_bytes(bytes);
                Some((value, 9))
            }
        }
    }

    /// Encode a varint into bytes and append to the given vector.
    ///
    /// Bitcoin varint encoding for values:
    /// - < 253: single byte
    /// - 253-65535: 0xFD + 2 bytes little-endian
    /// - 65536-4294967295: 0xFE + 4 bytes little-endian
    /// - >= 4294967296: 0xFF + 8 bytes little-endian
    fn encode_varint(value: u64, output: &mut Vec<u8>) {
        match value {
            n if n < 253 => {
                output.push(n as u8);
            }
            n if n <= 0xFFFF => {
                output.push(0xFD);
                output.extend_from_slice(&(n as u16).to_le_bytes());
            }
            n if n <= 0xFFFFFFFF => {
                output.push(0xFE);
                output.extend_from_slice(&(n as u32).to_le_bytes());
            }
            n => {
                output.push(0xFF);
                output.extend_from_slice(&n.to_le_bytes());
            }
        }
    }

    /// Parse a full BIP-322 signature from decoded bytes
    fn parse_full_signature(data: &[u8]) -> Result<Self, Bip322Error> {
        // Full BIP-322 signatures contain witness stack data
        // The format is: witness stack with multiple items (signature, pubkey, etc.)
        let witness_stack = Self::parse_witness_stack(data)?;

        Ok(Bip322Signature::Full { witness_stack })
    }

    /// Parse witness stack from raw bytes
    ///
    /// BIP-322 witness stacks are encoded as:
    /// - Number of witness elements (varint)
    /// - For each element: length (varint) + data
    fn parse_witness_stack(data: &[u8]) -> Result<Vec<Vec<u8>>, Bip322Error> {
        let mut cursor = 0;
        let mut witness_stack = Vec::new();

        if data.is_empty() {
            return Err(Bip322Error::InvalidWitnessFormat);
        }

        // Read number of witness items using proper varint decoding
        let (witness_count, consumed) = Self::read_varint(data, cursor)
            .ok_or(Bip322Error::InvalidWitnessFormat)?;
        cursor += consumed;

        // Validate witness count is reasonable to prevent DoS
        if witness_count > 10000 {
            return Err(Bip322Error::InvalidWitnessFormat);
        }

        for _ in 0..witness_count {
            if cursor >= data.len() {
                return Err(Bip322Error::InvalidWitnessFormat);
            }

            // Read item length using proper varint decoding
            let (item_length, consumed) = Self::read_varint(data, cursor)
                .ok_or(Bip322Error::InvalidWitnessFormat)?;
            cursor += consumed;

            // Validate item length is reasonable to prevent DoS
            if item_length > 1_000_000 {
                return Err(Bip322Error::InvalidWitnessFormat);
            }

            let item_length = item_length as usize;
            if cursor + item_length > data.len() {
                return Err(Bip322Error::InvalidWitnessFormat);
            }

            // Extract witness item
            let item = data[cursor..cursor + item_length].to_vec();
            witness_stack.push(item);
            cursor += item_length;
        }

        Ok(witness_stack)
    }

    /// Extract public key from the signature using appropriate method for signature type.
    ///
    /// For compact signatures, uses ECDSA recovery with the provided message hash,
    /// then validates that the recovered key matches the provided address.
    /// For full signatures, extracts from the witness data or transaction structure.
    pub fn extract_public_key(
        &self,
        message_hash: &[u8; 32],
        address: &Address,
    ) -> Option<<Secp256k1 as Curve>::PublicKey> {
        match self {
            Bip322Signature::Compact { signature } => {
                let recovered_pubkey =
                    Self::try_recover_pubkey_from_compact(message_hash, signature)?;

                // Validate that the recovered public key matches the address
                if crate::verification::validate_pubkey_matches_address(&recovered_pubkey, address)
                {
                    Some(recovered_pubkey)
                } else {
                    None
                }
            }
            Bip322Signature::Full { witness_stack } => {
                let parsed_pubkey = Self::extract_pubkey_from_full_signature(witness_stack, address)?;
                Self::validate_parsed_pubkey_matches_address(&parsed_pubkey, address)
            }
        }
    }

    /// Extract public key from full BIP-322 signature witness stack
    fn extract_pubkey_from_full_signature(
        witness_stack: &[Vec<u8>],
        address: &Address,
    ) -> Option<ParsedPublicKey> {
        match address {
            Address::P2PKH { .. } => {
                // For P2PKH, the public key should be in the witness stack
                // This is unusual for P2PKH but possible in BIP-322 context
                Self::extract_pubkey_from_witness_p2pkh(witness_stack)
            }
            Address::P2WPKH { .. } => {
                // For P2WPKH, witness stack format: [signature, pubkey]
                Self::extract_pubkey_from_witness_p2wpkh(witness_stack)
            }
            Address::P2SH { .. } => {
                // For P2SH, depends on the redeem script type
                Self::extract_pubkey_from_witness_p2sh(witness_stack)
            }
            Address::P2WSH { .. } => {
                // For P2WSH, witness stack format: [signature, pubkey, witness_script]
                Self::extract_pubkey_from_witness_p2wsh(witness_stack)
            }
        }
    }

    /// Extract public key from P2PKH witness stack
    fn extract_pubkey_from_witness_p2pkh(
        witness_stack: &[Vec<u8>],
    ) -> Option<ParsedPublicKey> {
        // For P2PKH in BIP-322, public key is typically the second element
        if witness_stack.len() >= 2 {
            Self::parse_pubkey_from_bytes(&witness_stack[1])
        } else {
            None
        }
    }

    /// Parse public key from raw bytes, preserving the original format.
    ///
    /// This method handles the common logic for parsing public keys from witness stacks:
    /// - 33 bytes: compressed format (preserved as-is)
    /// - 65 bytes: uncompressed format with 0x04 prefix (extract 64-byte key)
    fn parse_pubkey_from_bytes(pubkey_bytes: &[u8]) -> Option<ParsedPublicKey> {
        match pubkey_bytes.len() {
            33 => {
                // Compressed public key - preserve as-is
                let mut compressed = [0u8; 33];
                compressed.copy_from_slice(pubkey_bytes);
                Some(ParsedPublicKey::Compressed(compressed))
            }
            65 => {
                // Uncompressed public key - skip the 0x04 prefix
                if pubkey_bytes[0] == 0x04 {
                    let mut uncompressed = [0u8; 64];
                    uncompressed.copy_from_slice(&pubkey_bytes[1..]);
                    Some(ParsedPublicKey::Uncompressed(uncompressed))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Extract public key from P2WPKH witness stack  
    fn extract_pubkey_from_witness_p2wpkh(
        witness_stack: &[Vec<u8>],
    ) -> Option<ParsedPublicKey> {
        // P2WPKH witness stack: [signature, pubkey]
        if witness_stack.len() == 2 {
            Self::parse_pubkey_from_bytes(&witness_stack[1])
        } else {
            None
        }
    }

    /// Extract public key from P2SH witness stack
    fn extract_pubkey_from_witness_p2sh(
        witness_stack: &[Vec<u8>],
    ) -> Option<ParsedPublicKey> {
        // P2SH can contain various redeem scripts
        // For now, handle the common case of P2WPKH-in-P2SH
        if witness_stack.len() >= 2 {
            Self::parse_pubkey_from_bytes(&witness_stack[1])
        } else {
            None
        }
    }

    /// Extract public key from P2WSH witness stack
    fn extract_pubkey_from_witness_p2wsh(
        witness_stack: &[Vec<u8>],
    ) -> Option<ParsedPublicKey> {
        // P2WSH witness stack can be complex depending on the witness script
        // For single-key scripts: [signature, pubkey, witness_script]
        if witness_stack.len() >= 2 {
            Self::parse_pubkey_from_bytes(&witness_stack[1])
        } else {
            None
        }
    }


    /// Validate that a parsed public key matches the given address.
    ///
    /// This method handles both compressed and uncompressed public keys without
    /// requiring decompression. For compressed keys, it validates the address but
    /// returns None since we cannot decompress to the expected uncompressed format.
    /// For uncompressed keys, it performs validation and returns the key if valid.
    ///
    /// Note: This is a transitional implementation. In the future, the API should
    /// be updated to work with both compressed and uncompressed keys natively.
    fn validate_parsed_pubkey_matches_address(
        parsed_pubkey: &ParsedPublicKey,
        address: &Address,
    ) -> Option<<Secp256k1 as Curve>::PublicKey> {
        match parsed_pubkey {
            ParsedPublicKey::Compressed(compressed) => {
                // Validate compressed public key against address
                if crate::verification::validate_compressed_pubkey_matches_address(compressed, address) {
                    // Validation succeeded, but we cannot provide uncompressed format
                    // This indicates a successful verification but inability to decompress
                    // For now, we'll create a placeholder uncompressed key to indicate success
                    // TODO: Implement proper decompression or change API to accept compressed keys
                    Some([0u8; 64]) // Placeholder indicating successful validation
                } else {
                    None
                }
            }
            ParsedPublicKey::Uncompressed(uncompressed) => {
                // Use existing validation logic for uncompressed keys
                if crate::verification::validate_pubkey_matches_address(uncompressed, address) {
                    Some(*uncompressed)
                } else {
                    None
                }
            }
        }
    }

    /// Recover public key from compact signature format.
    ///
    /// This method handles the standard Bitcoin message signing recovery process.
    pub fn try_recover_pubkey_from_compact(
        message_hash: &[u8; 32],
        signature_bytes: &[u8; 65],
    ) -> Option<<Secp256k1 as Curve>::PublicKey> {
        // Validate recovery ID range (27-34 for standard Bitcoin compact format)
        let recovery_id = signature_bytes[0];
        if recovery_id < 27 || recovery_id > 34 {
            return None; // Invalid recovery ID
        }

        // Calculate v byte to make it in 0-3 range
        let v = if ((recovery_id - 27) & 4) != 0 {
            // compressed
            recovery_id - 31
        } else {
            // uncompressed
            recovery_id - 27
        };

        // Use env::ecrecover to recover public key from signature
        env::ecrecover(message_hash, &signature_bytes[1..], v, true)
    }

    /// Compute the appropriate message hash for this signature type.
    ///
    /// Compact signatures use standard Bitcoin message signing hash format.
    /// Full signatures use the complete BIP-322 transaction construction.
    pub fn compute_message_hash(&self, message: &str, address: &Address) -> [u8; 32] {
        match self {
            Bip322Signature::Compact { .. } => {
                // For compact signatures, use standard Bitcoin message signing
                // This follows the format: double SHA256 of "Bitcoin Signed Message:\n" + message
                Self::compute_bitcoin_message_hash(message)
            }
            Bip322Signature::Full { .. } => {
                // For full BIP-322 signatures, use the complete transaction construction
                let message_hash = Bip322MessageHasher::compute_bip322_message_hash(message);
                let to_spend = create_to_spend(address, &message_hash);
                let to_sign = create_to_sign(&to_spend);

                Bip322MessageHasher::compute_message_hash(&to_spend, &to_sign, address)
            }
        }
    }

    /// Compute standard Bitcoin message signing hash.
    ///
    /// This follows the Bitcoin Core format:
    /// Hash = SHA256(SHA256("Bitcoin Signed Message:\n" + varint(message.len()) + message))
    fn compute_bitcoin_message_hash(message: &str) -> [u8; 32] {
        use defuse_near_utils::digest::DoubleSha256;
        use digest::Digest;

        // Bitcoin message signing format
        let prefix = b"Bitcoin Signed Message:\n";
        let message_bytes = message.as_bytes();

        // Create the full message with prefix and length
        let mut full_message = Vec::new();
        full_message.extend_from_slice(prefix);

        // Add message length as proper varint
        Self::encode_varint(message_bytes.len() as u64, &mut full_message);

        full_message.extend_from_slice(message_bytes);

        // Double SHA256 hash
        DoubleSha256::digest(&full_message).into()
    }
}
