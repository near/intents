//! # Minimal Bitcoin Types for BIP-322 Implementation
//!
//! This module provides a minimal set of Bitcoin data structures and algorithms
//! specifically tailored for BIP-322 message verification. It focuses on:
//!
//! - **Address parsing**: P2PKH (Base58) and P2WPKH (Bech32) address formats
//! - **Transaction encoding**: Bitcoin transaction serialization for hashing
//! - **Script construction**: Basic Bitcoin script operations
//! - **NEAR SDK integration**: All cryptographic operations use NEAR host functions (SHA-256, RIPEMD-160)
//!
//! ## Design Principles
//!
//! 1. **Minimal Dependencies**: Only includes essential Bitcoin functionality
//! 2. **NEAR Optimized**: Uses `env::sha256_array()` and `env::ripemd160_array()` for all hash computations
//! 3. **MVP Focus**: Supports only P2PKH and P2WPKH for Phase 2-3
//! 4. **Gas Efficient**: Optimized for NEAR Protocol's gas model
//!
//! ## Supported Address Types
//!
//! - **P2PKH**: Legacy addresses starting with '1' (`Base58Check` encoded)
//! - **P2WPKH**: Segwit v0 addresses starting with 'bc1q' (Bech32 encoded)
//!
//! Future phases will add P2SH ('3' addresses) and P2WSH support.
//!
//! ## Key Components
//!
//! - `Address`: Bitcoin address representation with type detection
//! - `Transaction`: Bitcoin transaction structure for BIP-322
//! - `Witness`: Segwit witness stack for signature data
//! - `ScriptBuf`: Bitcoin script construction and storage
//! - Encoding functions: Transaction serialization for hash computation

use bech32::{Hrp, segwit};
use defuse_bip340::Double;
use digest::{Digest, FixedOutput, HashMarker, OutputSizeUser, Update};
use near_sdk::{env, near};
use serde_with::serde_as;

/// NEAR SDK SHA-256 implementation compatible with the `digest` crate traits.
///
/// This implementation uses NEAR SDK's `env::sha256_array()` function for
/// cryptographic operations, making it suitable for use in NEAR smart contracts
/// while being compatible with BIP340's `Double` and `Bip340TaggedDigest` functionality.
#[derive(Debug, Clone, Default)]
pub struct NearSha256 {
    buffer: Vec<u8>,
}

impl NearSha256 {
    /// Creates a new NEAR SHA-256 hasher instance.
    pub const fn new() -> Self {
        Self { buffer: Vec::new() }
    }
}

impl Update for NearSha256 {
    fn update(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }
}

impl OutputSizeUser for NearSha256 {
    type OutputSize = digest::consts::U32;
}

impl FixedOutput for NearSha256 {
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        let hash = env::sha256_array(&self.buffer);
        out.copy_from_slice(&hash);
    }
}

impl HashMarker for NearSha256 {}

// Note: Digest trait is automatically implemented for types that implement
// FixedOutput + Default + Update + HashMarker

/// Type alias for double SHA-256 using NEAR SDK functions.
///
/// This combines BIP340's `Double` wrapper with our NEAR SDK implementation
/// to provide Bitcoin's standard double SHA-256 hash function.
pub type NearDoubleSha256 = Double<NearSha256>;

/// Computes HASH160 (RIPEMD160(SHA256(data))) for Bitcoin address generation using NEAR SDK.
///
/// HASH160 is Bitcoin's standard address hash function used for:
/// - P2PKH address generation from public keys
/// - P2WPKH address generation from public keys
/// - Script hash computation for P2SH addresses
///
/// The algorithm: `RIPEMD160(SHA256(data))`
///
/// This implementation uses NEAR SDK's optimized host functions:
/// - `env::sha256_array()` for SHA-256 computation
/// - `env::ripemd160_array()` for RIPEMD-160 computation
///
/// # Arguments
///
/// * `data` - The input data to hash (typically a public key)
///
/// # Returns
///
/// A 20-byte HASH160 result computed using NEAR SDK host functions
pub fn hash160(data: &[u8]) -> [u8; 20] {
    // First pass: SHA256 using NEAR SDK host function
    let sha256_result = env::sha256_array(data);

    // Second pass: RIPEMD160 using NEAR SDK host function
    env::ripemd160_array(&sha256_result)
}

/// Bitcoin address representation optimized for BIP-322 verification.
///
/// This structure holds a parsed Bitcoin address with pre-computed data
/// needed for signature verification. It supports the two most common
/// address types used in modern Bitcoin transactions.
///
/// # Supported Formats
///
/// - **P2PKH**: Pay-to-Public-Key-Hash addresses starting with '1'
///   - Example: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"
///   - Uses Base58Check encoding with version byte 0x00
///   - Contains RIPEMD160(SHA256(pubkey)) hash
///
/// - **P2WPKH**: Pay-to-Witness-Public-Key-Hash addresses starting with 'bc1q'
///   - Example: "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l"
///   - Uses Bech32 encoding with witness version 0
///   - Contains the same pubkey hash as P2PKH but in witness format
///
/// # Fields
///
/// - `inner`: The original address string for reference
/// - `address_type`: Parsed address type (P2PKH or P2WPKH)
/// - `pubkey_hash`: The 20-byte hash for address validation (optional for MVP)
/// - `witness_program`: Segwit witness program data (for P2WPKH addresses)
#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Address {
    /// The parsed address type, determining verification method.
    ///
    /// This field determines which BIP-322 verification algorithm to use:
    /// - `P2PKH`: Uses legacy Bitcoin sighash algorithm
    /// - `P2WPKH`: Uses segwit v0 sighash algorithm
    pub address_type: AddressType,

    /// The 20-byte public key hash extracted from the address.
    ///
    /// For both P2PKH and P2WPKH, this contains RIPEMD160(SHA256(pubkey)).
    /// This field is used for address validation during signature verification.
    /// Marked with `#[serde(skip)]` to exclude from JSON serialization.
    #[serde(skip)]
    pub pubkey_hash: Option<[u8; 20]>,

    /// Segwit witness program data for P2WPKH addresses.
    ///
    /// Contains the witness version (0 for P2WPKH) and the program data
    /// (20-byte pubkey hash). Only populated for segwit addresses.
    /// Marked with `#[serde(skip)]` to exclude from JSON serialization.
    #[serde(skip)]
    pub witness_program: Option<WitnessProgram>,
}

/// Enumeration of supported Bitcoin address types.
///
/// This enum defines the address formats supported in the current MVP implementation.
/// Each type corresponds to a different signature verification algorithm.
#[near(serializers = [json])]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressType {
    /// Pay-to-Public-Key-Hash (legacy Bitcoin addresses).
    ///
    /// - Start with '1' on the mainnet
    /// - Use Base58Check encoding
    /// - Require legacy Bitcoin sighash algorithm for verification
    /// - Example: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"
    P2PKH,

    /// Pay-to-Witness-Public-Key-Hash (segwit v0 addresses).
    ///
    /// - Start with 'bc1q' on mainnet
    /// - Use Bech32 encoding
    /// - Require segwit v0 sighash algorithm for verification
    /// - Example: "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l"
    P2WPKH,

    /// Pay-to-Script-Hash (legacy Bitcoin script addresses).
    ///
    /// - Start with '3' on the mainnet
    /// - Use Base58Check encoding with version byte 0x05
    /// - Require legacy Bitcoin sighash algorithm for verification
    /// - Example: "3EktnHQD7RiAE6uzMj2ZifT9YgRrkSgzQX"
    P2SH,

    /// Pay-to-Witness-Script-Hash (segwit v0 script addresses).
    ///
    /// - Start with 'bc1q' on mainnet (but longer than P2WPKH)
    /// - Use Bech32 encoding with 32-byte witness program
    /// - Require segwit v0 sighash algorithm for verification
    /// - Example: "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3"
    P2WSH,
}

/// Parsed address data containing the essential cryptographic information.
///
/// This enum represents the different types of Bitcoin addresses after parsing,
/// extracting the essential hash or program data needed for signature verification.
/// Each variant contains the specific data needed for its address type.
#[derive(Debug, Clone)]
pub enum AddressData {
    /// Pay-to-Public-Key-Hash data containing the 20-byte hash of the public key.
    P2pkh { pubkey_hash: [u8; 20] },

    /// Pay-to-Script-Hash data containing the 20-byte hash of the redeem script.
    P2sh { script_hash: [u8; 20] },

    /// Pay-to-Witness-Public-Key-Hash data with the witness program.
    P2wpkh { witness_program: WitnessProgram },

    /// Pay-to-Witness-Script-Hash data with the witness program.
    P2wsh { witness_program: WitnessProgram },
}

/// Segwit witness program containing version and program data.
///
/// This structure represents the parsed witness program from a segwit address.
/// It contains the witness version (currently 0 for P2WPKH/P2WSH) and the
/// witness program bytes (20 bytes for P2WPKH, 32 bytes for P2WSH).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessProgram {
    /// Witness version (0 for current segwit, 1-16 for future versions)
    pub version: u8,
    /// Witness program bytes (20 bytes for P2WPKH, 32 bytes for P2WSH)
    pub program: Vec<u8>,
}

impl WitnessProgram {
    pub fn is_p2wpkh(&self) -> bool {
        self.version == 0 && self.program.len() == 20
    }

    pub fn is_p2wsh(&self) -> bool {
        self.version == 0 && self.program.len() == 32
    }
}

/// Bitcoin witness stack for storing signature and script data.
///
/// The witness stack is used in segwit transactions and BIP-322 signatures to store
/// signature data and scripts. The format depends on the address type:
/// - P2WPKH: [signature, pubkey]
/// - P2WSH: [signature, pubkey, witness_script]
/// - P2SH: [signature, pubkey, redeem_script]
#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Witness {
    stack: Vec<Vec<u8>>,
}

impl Default for Witness {
    fn default() -> Self {
        Self::new()
    }
}

impl Witness {
    pub const fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn nth(&self, index: usize) -> Option<&[u8]> {
        self.stack.get(index).map(Vec::as_slice)
    }

    /// Create a witness with the given stack elements (for testing)
    pub const fn from_stack(stack: Vec<Vec<u8>>) -> Self {
        Self { stack }
    }
}

impl Address {
    pub const fn assume_checked_ref(&self) -> &Self {
        self
    }

    /// Extracts address data with proper error handling for missing cryptographic data.
    ///
    /// Returns an error if required cryptographic data is missing for the address type:
    /// - P2PKH/P2SH addresses require `pubkey_hash`/`script_hash`
    /// - P2WPKH/P2WSH addresses require `witness_program`
    ///
    /// # Errors
    ///
    /// Returns `AddressError::MissingRequiredData` if the required cryptographic data
    /// is not present for the address type.
    pub fn to_address_data(&self) -> Result<AddressData, AddressError> {
        match self.address_type {
            AddressType::P2PKH => {
                let pubkey_hash = self.pubkey_hash.ok_or(AddressError::MissingRequiredData)?;
                Ok(AddressData::P2pkh { pubkey_hash })
            }
            AddressType::P2SH => {
                let script_hash = self.pubkey_hash.ok_or(AddressError::MissingRequiredData)?;
                Ok(AddressData::P2sh { script_hash })
            }
            AddressType::P2WPKH => {
                let witness_program = self
                    .witness_program
                    .clone()
                    .ok_or(AddressError::MissingRequiredData)?;
                Ok(AddressData::P2wpkh { witness_program })
            }
            AddressType::P2WSH => {
                let witness_program = self
                    .witness_program
                    .clone()
                    .ok_or(AddressError::MissingRequiredData)?;
                Ok(AddressData::P2wsh { witness_program })
            }
        }
    }

    /// Generates the script pubkey for this address.
    ///
    /// # Errors
    ///
    /// Returns `AddressError::MissingRequiredData` if required cryptographic data
    /// is missing for the address type.
    pub fn script_pubkey(&self) -> Result<ScriptBuf, AddressError> {
        match self.address_type {
            AddressType::P2PKH => {
                // P2PKH script: OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG
                let pubkey_hash = self.pubkey_hash.ok_or(AddressError::MissingRequiredData)?;
                let mut script = Vec::new();
                script.push(0x76); // OP_DUP
                script.push(0xa9); // OP_HASH160
                script.push(20); // Push 20 bytes
                script.extend_from_slice(&pubkey_hash);
                script.push(0x88); // OP_EQUALVERIFY
                script.push(0xac); // OP_CHECKSIG
                Ok(ScriptBuf { inner: script })
            }
            AddressType::P2SH => {
                // P2SH script: OP_HASH160 <script_hash> OP_EQUAL
                let script_hash = self.pubkey_hash.ok_or(AddressError::MissingRequiredData)?;
                let mut script = Vec::new();
                script.push(0xa9); // OP_HASH160
                script.push(20); // Push 20 bytes
                script.extend_from_slice(&script_hash);
                script.push(0x87); // OP_EQUAL
                Ok(ScriptBuf { inner: script })
            }
            AddressType::P2WPKH => {
                // P2WPKH script: OP_0 <20-byte-pubkey-hash>
                let pubkey_hash = self.pubkey_hash.ok_or(AddressError::MissingRequiredData)?;
                let mut script = Vec::new();
                script.push(0x00); // OP_0
                script.push(20); // Push 20 bytes
                script.extend_from_slice(&pubkey_hash);
                Ok(ScriptBuf { inner: script })
            }
            AddressType::P2WSH => {
                // P2WSH script: OP_0 <32-byte-script-hash>
                let witness_program = self
                    .witness_program
                    .as_ref()
                    .ok_or(AddressError::MissingRequiredData)?;

                if witness_program.program.len() != 32 {
                    return Err(AddressError::InvalidWitnessProgram);
                }

                let mut script = Vec::new();
                script.push(0x00); // OP_0
                script.push(32); // Push 32 bytes
                script.extend_from_slice(&witness_program.program);
                Ok(ScriptBuf { inner: script })
            }
        }
    }
}

/// Implementation of address parsing from the string format.
///
/// This implementation supports parsing the two most common Bitcoin address formats
/// with full validation including checksum verification.
impl std::str::FromStr for Address {
    type Err = AddressError;

    /// Parses a Bitcoin address string into an `Address` structure.
    ///
    /// This method performs comprehensive validation including
    /// - Format detection (P2PKH, P2SH, P2WPKH, P2WSH)
    /// - Encoding validation (`Base58Check` vs Bech32)
    /// - Checksum verification
    /// - Length validation
    /// - Network validation (mainnet only)
    ///
    /// # Arguments
    ///
    /// * `s` - The address string to parse
    ///
    /// # Returns
    ///
    /// - `Ok(Address)` if parsing succeeds with valid checksum
    /// - `Err(AddressError)` if parsing fails for any reason
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let p2pkh: Address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse()?;
    /// let p2sh: Address = "3EktnHQD7RiAE6uzMj2ZifT9YgRrkSgzQX".parse()?;
    /// let p2wpkh: Address = "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l".parse()?;
    /// let p2wsh: Address = "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3".parse()?;
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // P2PKH (Pay-to-Public-Key-Hash) address parsing
        // These are legacy Bitcoin addresses starting with '1' on the mainnet
        if s.starts_with('1') {
            // Decode the Base58Check encoded address
            // Base58Check = Base58(version + payload + checksum)
            let decoded = bs58::decode(s)
                .into_vec()
                .map_err(|_| AddressError::InvalidBase58)?;

            // P2PKH addresses must be exactly 25 bytes:
            // 1 byte version + 20 bytes pubkey_hash + 4 bytes checksum
            if decoded.len() != 25 {
                return Err(AddressError::InvalidLength);
            }

            // Verify version byte for P2PKH mainnet addresses
            // 0x00 = P2PKH mainnet, 0x6f = P2PKH testnet (not supported)
            if decoded[0] != 0x00 {
                return Err(AddressError::InvalidBase58);
            }

            // Extract the 20-byte public key hash
            // This is RIPEMD160(SHA256(pubkey)) from bytes 1-20
            let mut pubkey_hash = [0u8; 20];
            pubkey_hash.copy_from_slice(&decoded[1..21]);

            // Verify Base58Check checksum (last 4 bytes)
            // Checksum = first 4 bytes of double_sha256(version + pubkey_hash)
            let payload = &decoded[..21]; // version + pubkey_hash
            let checksum = &decoded[21..25]; // provided checksum
            let computed_checksum: [u8; 32] = NearDoubleSha256::digest(payload).into();
            if &computed_checksum[..4] != checksum {
                return Err(AddressError::InvalidBase58);
            }

            Ok(Self {
                address_type: AddressType::P2PKH,
                pubkey_hash: Some(pubkey_hash),
                witness_program: None,
            })
        }
        // P2SH (Pay-to-Script-Hash) address parsing
        // These are legacy Bitcoin script addresses starting with '3' on the mainnet
        else if s.starts_with('3') {
            // Decode the Base58Check encoded address
            // Base58Check = Base58(version + payload + checksum)
            let decoded = bs58::decode(s)
                .into_vec()
                .map_err(|_| AddressError::InvalidBase58)?;

            // P2SH addresses must be exactly 25 bytes:
            // 1 byte version + 20 bytes script_hash + 4 bytes checksum
            if decoded.len() != 25 {
                return Err(AddressError::InvalidLength);
            }

            // Verify version byte for P2SH mainnet addresses
            // 0x05 = P2SH mainnet, 0xc4 = P2SH testnet (not supported)
            if decoded[0] != 0x05 {
                return Err(AddressError::InvalidBase58);
            }

            // Extract the 20-byte script hash
            // This is RIPEMD160(SHA256(script)) from bytes 1-20
            let mut script_hash = [0u8; 20];
            script_hash.copy_from_slice(&decoded[1..21]);

            // Verify Base58Check checksum (last 4 bytes)
            // Checksum = first 4 bytes of double_sha256(version + script_hash)
            let payload = &decoded[..21]; // version + script_hash
            let checksum = &decoded[21..25]; // provided checksum
            let computed_checksum: [u8; 32] = NearDoubleSha256::digest(payload).into();
            if &computed_checksum[..4] != checksum {
                return Err(AddressError::InvalidBase58);
            }

            Ok(Self {
                address_type: AddressType::P2SH,
                pubkey_hash: Some(script_hash), // Store script hash in the pubkey_hash field
                witness_program: None,
            })
        }
        // P2WPKH/P2WSH (Pay-to-Witness-Public-Key-Hash/Script-Hash) address parsing
        // These are segwit addresses starting with 'bc1' on the mainnet
        else if s.starts_with("bc1") {
            // Decode the Bech32 encoded address with full validation
            // This includes proper checksum verification and format validation
            let (witness_version, witness_program) = decode_bech32(s)?;

            // We only support segwit version 0
            if witness_version != 0 {
                return Err(AddressError::UnsupportedFormat);
            }

            // Distinguish between P2WPKH (20 bytes) and P2WSH (32 bytes)
            match witness_program.len() {
                20 => {
                    // P2WPKH: 20-byte public key hash
                    let mut pubkey_hash = [0u8; 20];
                    pubkey_hash.copy_from_slice(&witness_program);

                    Ok(Self {
                        address_type: AddressType::P2WPKH,
                        pubkey_hash: Some(pubkey_hash),
                        witness_program: Some(WitnessProgram {
                            version: witness_version,
                            program: witness_program,
                        }),
                    })
                }
                32 => {
                    // P2WSH: 32-byte script hash
                    Ok(Self {
                        address_type: AddressType::P2WSH,
                        pubkey_hash: None, // P2WSH doesn't have a pubkey hash
                        witness_program: Some(WitnessProgram {
                            version: witness_version,
                            program: witness_program,
                        }),
                    })
                }
                _ => {
                    // Invalid witness program length for segwit v0
                    Err(AddressError::InvalidWitnessProgram)
                }
            }
        } else {
            // Unsupported address format
            // Currently supports:
            // - P2PKH addresses starting with '1'
            // - P2SH addresses starting with '3'
            // - P2WPKH addresses starting with 'bc1q' (20-byte witness program)
            // - P2WSH addresses starting with 'bc1q' (32-byte witness program)
            // Future: other segwit versions, testnet addresses
            Err(AddressError::UnsupportedFormat)
        }
    }
}

/// Errors that can occur during Bitcoin address parsing.
///
/// This enum provides detailed error information for different failure modes
/// in address parsing, allowing for specific error handling and user feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressError {
    /// Invalid `Base58Check` encoding (for P2PKH addresses).
    ///
    /// This includes:
    /// - Invalid characters in the Base58 alphabet
    /// - Checksum validation failures
    /// - Invalid version bytes
    InvalidBase58,

    /// Invalid address length (typically for P2PKH addresses).
    ///
    /// P2PKH addresses must be exactly 25 bytes when decoded:
    /// 1 byte version + 20 bytes `pubkey_hash` + 4 bytes checksum
    InvalidLength,

    /// Invalid witness program format or length.
    ///
    /// This includes:
    /// - Witness programs with invalid lengths for their version
    /// - Malformed witness data
    InvalidWitnessProgram,

    /// Unsupported address format.
    ///
    /// Currently supports only:
    /// - P2PKH addresses starting with '1'
    /// - P2WPKH/P2WSH addresses starting with 'bc1'
    UnsupportedFormat,

    /// Invalid Bech32 encoding (for segwit addresses).
    ///
    /// This includes:
    /// - Invalid characters in the Bech32 alphabet
    /// - Checksum validation failures
    /// - Invalid HRP (Human Readable Part)
    /// - Malformed segwit data
    InvalidBech32,

    /// Missing required data for address type.
    ///
    /// This occurs when:
    /// - P2PKH/P2SH addresses are missing `pubkey_hash`/`script_hash`
    /// - P2WPKH/P2WSH addresses are missing `witness_program`
    MissingRequiredData,
}

impl std::fmt::Display for AddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidBase58 => write!(f, "Invalid base58 encoding"),
            Self::InvalidLength => write!(f, "Invalid address length"),
            Self::InvalidWitnessProgram => write!(f, "Invalid witness program"),
            Self::UnsupportedFormat => write!(f, "Unsupported address format"),
            Self::InvalidBech32 => write!(f, "Invalid bech32 encoding"),
            Self::MissingRequiredData => {
                write!(f, "Missing required cryptographic data for address type")
            }
        }
    }
}

impl std::error::Error for AddressError {}

/// Full Bech32 decoder for Bitcoin segwit addresses using the bech32 crate.
///
/// This implementation provides complete Bech32 decoding with proper checksum validation
/// and error detection as specified in BIP-173. It supports all segwit address types
/// on Bitcoin mainnet.
///
/// # Algorithm Overview
///
/// 1. Parse the HRP (Human Readable Part) - should be "bc" for mainnet
/// 2. Decode the data part using proper Bech32 decoding algorithm
/// 3. Validate the Bech32 checksum (6-character suffix)
/// 4. Convert the witness version and program from 5-bit to 8-bit encoding
/// 5. Validate witness version and program length constraints
///
/// # Arguments
///
/// * `addr` - The bech32 address string to decode
///
/// # Returns
///
/// A tuple containing:
/// - `witness_version`: The segwit version (0 for P2WPKH/P2WSH)
/// - `witness_program`: The witness program bytes
///
/// # Errors
///
/// Returns `AddressError::InvalidBech32` for any decoding failures including
/// - Invalid characters in the address
/// - Checksum validation failures  
/// - Invalid witness version or program length
/// - Non-mainnet HRP (not "bc")
fn decode_bech32(addr: &str) -> Result<(u8, Vec<u8>), AddressError> {
    // Parse the segwit address using the bech32 crate's segwit module
    // This handles the complete segwit address decoding including checksum validation
    let (hrp, witness_version, witness_program) =
        segwit::decode(addr).map_err(|_| AddressError::InvalidBech32)?;

    // Verify this is a Bitcoin mainnet address (HRP = "bc")
    // Testnet would be "tb", regtest would be "bcrt"
    if hrp != Hrp::parse("bc").unwrap() {
        return Err(AddressError::InvalidBech32);
    }

    // Validate witness program length constraints per BIP-141
    // The bech32 crate should already validate these, but we double-check
    match witness_version.to_u8() {
        0 => {
            // Segwit v0: program must be 20 bytes (P2WPKH) or 32 bytes (P2WSH)
            if witness_program.len() != 20 && witness_program.len() != 32 {
                return Err(AddressError::InvalidWitnessProgram);
            }
        }
        1..=16 => {
            // Future segwit versions: program must be 2-40 bytes per BIP-141
            if witness_program.len() < 2 || witness_program.len() > 40 {
                return Err(AddressError::InvalidWitnessProgram);
            }
        }
        _ => return Err(AddressError::InvalidBech32),
    }

    Ok((witness_version.to_u8(), witness_program))
}

/// Script buffer
#[derive(Debug, Clone)]
pub struct ScriptBuf {
    inner: Vec<u8>,
}

impl Default for ScriptBuf {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptBuf {
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

/// Transaction ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Txid([u8; 32]);

impl Txid {
    pub const fn all_zeros() -> Self {
        Self([0u8; 32])
    }

    pub const fn from_byte_array(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Transaction output point
#[derive(Debug, Clone)]
pub struct OutPoint {
    pub txid: Txid,
    pub vout: u32,
}

impl OutPoint {
    pub const fn new(txid: Txid, vout: u32) -> Self {
        Self { txid, vout }
    }
}

/// Bitcoin transaction input referencing a previous output.
///
/// A transaction input spends a previous transaction output by referencing
/// its transaction ID and output index, along with providing the necessary
/// signature data to prove ownership.
#[derive(Debug, Clone)]
pub struct TxIn {
    /// Reference to the output being spent
    pub previous_output: OutPoint,
    /// Script signature (legacy) or empty for segwit
    pub script_sig: ScriptBuf,
    /// Sequence number for transaction replacement/timelock
    pub sequence: Sequence,
    /// Witness data for segwit transactions
    pub witness: Witness,
}

/// Bitcoin transaction output containing value and locking script.
///
/// Each output specifies an amount of bitcoin and the conditions (script)
/// that must be satisfied to spend those coins in a future transaction.
#[derive(Debug, Clone)]
pub struct TxOut {
    /// The value/amount of bitcoin in this output
    pub value: Amount,
    pub script_pubkey: ScriptBuf,
}

/// Bitcoin transaction containing inputs, outputs, and metadata.
///
/// A transaction represents a transfer of bitcoin from inputs (references to previous
/// outputs) to new outputs. It includes version information and a lock time that
/// can be used for time-based transaction validation.
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Transaction format version
    pub version: Version,
    /// Earliest time/block when transaction can be included
    pub lock_time: LockTime,
    /// Transaction inputs (coins being spent)
    pub input: Vec<TxIn>,
    /// Transaction outputs (new coin allocations)
    pub output: Vec<TxOut>,
}

/// Bitcoin amount representation in satoshis.
///
/// Bitcoin amounts are represented as 64-bit unsigned integers in satoshis,
/// where 1 BTC = 100,000,000 satoshis. This provides sufficient precision
/// for all Bitcoin monetary operations.
#[derive(Debug, Clone, Copy)]
pub struct Amount(u64);

impl Amount {
    pub const ZERO: Self = Self(0);
}

/// Version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version(pub i32);

/// Lock time
#[derive(Debug, Clone, Copy)]
pub struct LockTime(u32);

impl LockTime {
    pub const ZERO: Self = Self(0);
}

/// Sequence
#[derive(Debug, Clone, Copy)]
pub struct Sequence(u32);

impl Sequence {
    pub const ZERO: Self = Self(0);
}

/// Consensus encodable trait
pub trait Encodable {
    fn consensus_encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error>;
}

impl Encodable for Transaction {
    fn consensus_encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let mut len = 0;

        // Check if any input has witness data
        let has_witness = self
            .input
            .iter()
            .any(|input| !input.witness.stack.is_empty());

        // Version (4 bytes, little-endian)
        len += writer.write(&self.version.0.to_le_bytes())?;

        // If witness data exists, write marker and flag bytes
        if has_witness {
            len += writer.write(&[0x00])?; // Marker byte
            len += writer.write(&[0x01])?; // Flag byte
        }

        // Input count (compact size)
        len += write_compact_size(writer, try_into_io::<usize, u64>(self.input.len())?)?;

        // Inputs
        for input in &self.input {
            // Previous output (36 bytes)
            len += writer.write(&input.previous_output.txid.0)?;
            len += writer.write(&input.previous_output.vout.to_le_bytes())?;

            // Script sig
            len += write_compact_size(
                writer,
                try_into_io::<usize, u64>(input.script_sig.inner.len())?,
            )?;
            len += writer.write(&input.script_sig.inner)?;

            // Sequence (4 bytes)
            len += writer.write(&input.sequence.0.to_le_bytes())?;
        }

        // Output count
        len += write_compact_size(writer, try_into_io::<usize, u64>(self.output.len())?)?;

        // Outputs
        for output in &self.output {
            // Value (8 bytes, little-endian)
            len += writer.write(&output.value.0.to_le_bytes())?;

            // Script pubkey
            len += write_compact_size(
                writer,
                try_into_io::<usize, u64>(output.script_pubkey.inner.len())?,
            )?;
            len += writer.write(&output.script_pubkey.inner)?;
        }

        // If witness data exists, serialize the witness for each input
        if has_witness {
            for input in &self.input {
                // Write witness stack size
                len += write_compact_size(
                    writer,
                    try_into_io::<usize, u64>(input.witness.stack.len())?,
                )?;

                // Write each witness item
                for witness_item in &input.witness.stack {
                    len +=
                        write_compact_size(writer, try_into_io::<usize, u64>(witness_item.len())?)?;
                    len += writer.write(witness_item)?;
                }
            }
        }

        // Lock time (4 bytes)
        len += writer.write(&self.lock_time.0.to_le_bytes())?;

        Ok(len)
    }
}

/// Helper function to convert between numeric types with proper error handling for IO operations.
///
/// This function is used throughout the encoding logic to safely convert between numeric types
/// (e.g., usize to u64, u64 to u32) while providing consistent error handling.
fn try_into_io<T, U>(value: T) -> Result<U, std::io::Error>
where
    T: TryInto<U>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    value
        .try_into()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn write_compact_size<W: std::io::Write>(writer: &mut W, n: u64) -> Result<usize, std::io::Error> {
    if n < 0xfd {
        writer.write_all(&[try_into_io::<u64, u8>(n)?])?;
        Ok(1)
    } else if n <= 0xffff {
        writer.write_all(&[0xfd])?;
        writer.write_all(&try_into_io::<u64, u16>(n)?.to_le_bytes())?;
        Ok(3)
    } else if n <= 0xffffffff {
        writer.write_all(&[0xfe])?;
        writer.write_all(&try_into_io::<u64, u32>(n)?.to_le_bytes())?;
        Ok(5)
    } else {
        writer.write_all(&[0xff])?;
        writer.write_all(&n.to_le_bytes())?;
        Ok(9)
    }
}

/// Script builder
pub struct ScriptBuilder {
    inner: Vec<u8>,
}

impl Default for ScriptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptBuilder {
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    #[must_use]
    pub fn push_opcode(mut self, opcode: u8) -> Self {
        self.inner.push(opcode);
        self
    }

    #[must_use]
    pub fn push_slice(mut self, data: &[u8]) -> Self {
        if data.len() <= 75 {
            self.inner
                .push(u8::try_from(data.len()).expect("data length fits in u8"));
        } else {
            panic!("Large pushdata not implemented");
        }
        self.inner.extend_from_slice(data);
        self
    }

    pub fn into_script(self) -> ScriptBuf {
        ScriptBuf { inner: self.inner }
    }
}

// Op codes
pub const OP_0: u8 = 0x00;
pub const OP_DUP: u8 = 0x76;
pub const OP_HASH160: u8 = 0xa9;
pub const OP_EQUALVERIFY: u8 = 0x88;
pub const OP_CHECKSIG: u8 = 0xac;
pub const OP_RETURN: u8 = 0x6a;

// Signature hash cache (simplified)
pub struct SighashCache {
    tx: Transaction,
}

impl SighashCache {
    pub const fn new(tx: Transaction) -> Self {
        Self { tx }
    }

    /// Encodes the BIP-143 sighash preimage for segwit v0 signature verification.
    ///
    /// This function implements the complete BIP-143 sighash algorithm for segwit v0
    /// transactions, creating the exact preimage that gets double-SHA256 hashed
    /// for signature verification.
    ///
    /// # BIP-143 Sighash Preimage Format
    ///
    /// The preimage consists of the following fields in order:
    /// 1. version (4 bytes)
    /// 2. hashPrevouts (32 bytes) - double SHA256 of all outpoints
    /// 3. hashSequence (32 bytes) - double SHA256 of all sequence numbers  
    /// 4. outpoint (36 bytes) - the specific input's outpoint being signed
    /// 5. scriptCode (variable) - with compact size prefix
    /// 6. amount (8 bytes) - value of the output being spent
    /// 7. sequence (4 bytes) - sequence of the input being signed
    /// 8. hashOutputs (32 bytes) - double SHA256 of all outputs
    /// 9. locktime (4 bytes)
    /// 10. `sighash_type` (4 bytes) - as little-endian integer
    pub fn segwit_v0_encode_signing_data_to<W: std::io::Write>(
        &mut self,
        writer: &mut W,
        input_index: usize,
        script_code: &ScriptBuf,
        value: Amount,
        sighash_type: EcdsaSighashType,
    ) -> Result<(), std::io::Error> {
        // 1. Transaction version (4 bytes, little-endian)
        writer.write_all(&self.tx.version.0.to_le_bytes())?;

        // 2. hashPrevouts (32 bytes) - double SHA256 of all outpoints
        let hash_prevouts = self.compute_hash_prevouts();
        writer.write_all(&hash_prevouts)?;

        // 3. hashSequence (32 bytes) - double SHA256 of all sequence numbers
        let hash_sequence = self.compute_hash_sequence();
        writer.write_all(&hash_sequence)?;

        // 4. Outpoint (36 bytes) - the specific input's outpoint being signed
        if input_index >= self.tx.input.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Input index out of bounds",
            ));
        }
        let input = &self.tx.input[input_index];
        writer.write_all(&input.previous_output.txid.0)?;
        writer.write_all(&input.previous_output.vout.to_le_bytes())?;

        // 5. scriptCode (variable length with compact size prefix)
        write_compact_size(writer, try_into_io::<usize, u64>(script_code.inner.len())?)?;
        writer.write_all(&script_code.inner)?;

        // 6. amount (8 bytes, little-endian) - value of the output being spent
        writer.write_all(&value.0.to_le_bytes())?;

        // 7. sequence (4 bytes, little-endian) - sequence of the input being signed
        writer.write_all(&input.sequence.0.to_le_bytes())?;

        // 8. hashOutputs (32 bytes) - double SHA256 of all outputs
        let hash_outputs = self.compute_hash_outputs()?;
        writer.write_all(&hash_outputs)?;

        // 9. locktime (4 bytes, little-endian)
        writer.write_all(&self.tx.lock_time.0.to_le_bytes())?;

        // 10. sighash_type (4 bytes, little-endian)
        writer.write_all(&u32::from(u8::from(sighash_type)).to_le_bytes())?;

        Ok(())
    }

    /// Computes hashPrevouts as specified in BIP-143.
    ///
    /// `hashPrevouts` = `double_sha256(all outpoints concatenated)`
    /// Each outpoint is 36 bytes: txid (32 bytes) + vout (4 bytes little-endian)
    fn compute_hash_prevouts(&self) -> [u8; 32] {
        let mut outpoints_data = Vec::new();
        for input in &self.tx.input {
            outpoints_data.extend_from_slice(&input.previous_output.txid.0);
            outpoints_data.extend_from_slice(&input.previous_output.vout.to_le_bytes());
        }
        NearDoubleSha256::digest(&outpoints_data).into()
    }

    /// Computes hashSequence as specified in BIP-143.
    ///
    /// `hashSequence` = `double_sha256(all sequence numbers concatenated)`
    /// Each sequence is 4 bytes little-endian
    fn compute_hash_sequence(&self) -> [u8; 32] {
        let mut sequence_data = Vec::new();
        for input in &self.tx.input {
            sequence_data.extend_from_slice(&input.sequence.0.to_le_bytes());
        }
        NearDoubleSha256::digest(&sequence_data).into()
    }

    /// Computes hashOutputs as specified in BIP-143.
    ///
    /// `hashOutputs` = `double_sha256(all outputs concatenated)`
    /// Each output is: value (8 bytes little-endian) + scriptPubKey (variable length with compact size prefix)
    fn compute_hash_outputs(&self) -> Result<[u8; 32], std::io::Error> {
        let mut outputs_data = Vec::new();
        for output in &self.tx.output {
            outputs_data.extend_from_slice(&output.value.0.to_le_bytes());
            // Write scriptPubKey with the compact size prefix
            let script_len = try_into_io::<usize, u64>(output.script_pubkey.inner.len())?;
            let mut compact_size_bytes = Vec::new();
            write_compact_size(&mut compact_size_bytes, script_len)
                .expect("Writing to Vec should not fail");
            outputs_data.extend_from_slice(&compact_size_bytes);
            outputs_data.extend_from_slice(&output.script_pubkey.inner);
        }
        Ok(NearDoubleSha256::digest(&outputs_data).into())
    }

    /// Encodes the legacy sighash preimage for P2PKH and P2SH signature verification.
    ///
    /// This function implements the original Bitcoin sighash algorithm used before segwit.
    /// The legacy sighash is simpler than BIP-143 but has known vulnerabilities like
    /// quadratic scaling behavior.
    ///
    /// # Legacy Sighash Preimage Format
    ///
    /// The preimage consists of the following fields in order:
    /// 1. version (4 bytes)
    /// 2. inputs with modified scripts
    /// 3. outputs
    /// 4. locktime (4 bytes)
    /// 5. `sighash_type` (4 bytes)
    ///
    /// For `SIGHASH_ALL` (the only type we support), all inputs and outputs are included.
    pub fn legacy_encode_signing_data_to<W: std::io::Write>(
        &mut self,
        writer: &mut W,
        input_index: usize,
        script_code: &ScriptBuf,
        sighash_type: EcdsaSighashType,
    ) -> Result<(), std::io::Error> {
        // 1. Transaction version (4 bytes, little-endian)
        writer.write_all(&self.tx.version.0.to_le_bytes())?;

        // 2. Number of inputs (compact size)
        let input_count = try_into_io::<usize, u64>(self.tx.input.len())?;
        write_compact_size(writer, input_count)?;

        // 3. Inputs with script modifications
        for (i, input) in self.tx.input.iter().enumerate() {
            // Write outpoint (txid + vout)
            writer.write_all(&input.previous_output.txid.0)?;
            writer.write_all(&input.previous_output.vout.to_le_bytes())?;

            // For legacy sighash, only the input being signed gets the script_code
            // All other inputs get empty scripts
            if i == input_index {
                // Use the provided script_code for the input being signed
                let script_len = try_into_io::<usize, u64>(script_code.inner.len())?;
                write_compact_size(writer, script_len)?;
                writer.write_all(&script_code.inner)?;
            } else {
                // Empty script for other inputs
                write_compact_size(writer, 0u64)?;
            }

            // Write sequence
            writer.write_all(&input.sequence.0.to_le_bytes())?;
        }

        // 4. Number of outputs (compact size)
        let output_count = try_into_io::<usize, u64>(self.tx.output.len())?;
        write_compact_size(writer, output_count)?;

        // 5. All outputs (for SIGHASH_ALL)
        for output in &self.tx.output {
            writer.write_all(&output.value.0.to_le_bytes())?;
            let script_len = try_into_io::<usize, u64>(output.script_pubkey.inner.len())?;
            write_compact_size(writer, script_len)?;
            writer.write_all(&output.script_pubkey.inner)?;
        }

        // 6. Locktime (4 bytes, little-endian)
        writer.write_all(&self.tx.lock_time.0.to_le_bytes())?;

        // 7. Sighash type (4 bytes, little-endian)
        let sighash_value = match sighash_type {
            EcdsaSighashType::All => 0x01u32,
        };
        writer.write_all(&sighash_value.to_le_bytes())?;

        Ok(())
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum EcdsaSighashType {
    All = 0x01,
}

impl From<EcdsaSighashType> for u8 {
    fn from(value: EcdsaSighashType) -> Self {
        match value {
            EcdsaSighashType::All => 0x01u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use rstest::rstest;

    /// Test that our NEAR SDK double hash using BIP340 Double produces expected results
    #[rstest]
    #[case(b"", hex!("5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456"))]
    #[case(b"hello", hex!("9595c9df90075148eb06860365df33584b75bff782a510c6cd4883a419833d50"))]
    fn test_near_double_sha256_bip340(#[case] input: &[u8], #[case] expected: [u8; 32]) {
        assert_eq!(NearDoubleSha256::digest(input), expected.into());
    }

    /// Test BIP340 tagged hash functionality using NEAR SDK
    #[rstest]
    #[case(b"BIP0340/challenge", b"test_data")]
    #[case(b"TapLeaf", b"script")]
    fn test_bip340_tagged_hash_near(#[case] tag: &[u8], #[case] data: &[u8]) {
        use defuse_bip340::Bip340TaggedDigest;

        // Use BIP340 tagged digest trait with NEAR SDK implementation
        let result = NearSha256::tagged(tag).chain_update(data).finalize();

        // Should produce a valid 32-byte hash
        assert_eq!(result.len(), 32);

        // Test that the tagged hash follows the BIP340 pattern
        let expected = {
            let tag_hash = NearSha256::digest(tag);
            NearSha256::new()
                .chain_update(tag_hash)
                .chain_update(tag_hash)
                .chain_update(data)
                .finalize()
        };
        assert_eq!(result, expected);
    }

    /// Test NEAR SHA256 basic functionality
    #[rstest]
    #[case(b"")]
    #[case(b"hello")]
    #[case(b"bitcoin")]
    fn test_near_sha256_basic(#[case] input: &[u8]) {
        let result = NearSha256::digest(input);
        assert_eq!(result.len(), 32);

        // Test that it matches what we get from incremental updates
        let incremental = NearSha256::new().chain_update(input).finalize();
        assert_eq!(result, incremental);
    }

    /// Test address parsing with different types
    #[rstest]
    #[case("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa", AddressType::P2PKH)]
    #[case("3EktnHQD7RiAE6uzMj2ZifT9YgRrkSgzQX", AddressType::P2SH)]
    #[case("bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l", AddressType::P2WPKH)]
    fn test_address_type_detection(#[case] addr_str: &str, #[case] expected_type: AddressType) {
        let addr: Address = addr_str.parse().expect("Valid address");
        assert_eq!(addr.address_type, expected_type);
    }
}
