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
//! - **P2PKH**: Legacy addresses starting with '1' (Base58Check encoded)
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

use near_sdk::{near, env};
use serde_with::serde_as;
use bech32::{Hrp, segwit};

/// Computes double SHA-256 hash using NEAR SDK cryptographic functions.
/// 
/// Double SHA-256 is Bitcoin's standard hash function used for:
/// - Transaction IDs (TXID computation)
/// - Block hashes
/// - Address checksums in Base58Check encoding
/// - Merkle tree construction
/// 
/// The algorithm: `SHA256(SHA256(data))`
/// 
/// # Arguments
/// 
/// * `data` - The input data to hash
/// 
/// # Returns
/// 
/// A 32-byte double SHA-256 hash computed using NEAR SDK's `env::sha256_array()`
pub fn double_sha256(data: &[u8]) -> [u8; 32] {
    // First SHA-256 pass using NEAR SDK
    let first_hash = env::sha256_array(data);
    
    // Second SHA-256 pass using NEAR SDK
    env::sha256_array(&first_hash)
}

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
    /// The original address string as provided by the user.
    /// 
    /// This is kept for reference and debugging purposes. Examples:
    /// - P2PKH: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"
    /// - P2WPKH: "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l"
    pub inner: String,
    
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressType {
    /// Pay-to-Public-Key-Hash (legacy Bitcoin addresses).
    /// 
    /// - Start with '1' on mainnet
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
    /// - Start with '3' on mainnet
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
    P2WSH
}

#[derive(Debug, Clone)]
pub enum AddressData {
    P2pkh { pubkey_hash: [u8; 20] },
    P2sh { script_hash: [u8; 20] },
    P2wpkh { witness_program: WitnessProgram },
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

/// Minimal Witness implementation
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
        self.stack.get(index).map(|v| v.as_slice())
    }
}

impl Address {
    pub fn assume_checked_ref(&self) -> &Self {
        self
    }
    
    pub fn to_address_data(&self) -> AddressData {
        match self.address_type {
            AddressType::P2PKH => {
                AddressData::P2pkh { 
                    pubkey_hash: self.pubkey_hash.unwrap_or([0u8; 20])
                }
            },
            AddressType::P2SH => {
                AddressData::P2sh { 
                    script_hash: self.pubkey_hash.unwrap_or([0u8; 20])
                }
            },
            AddressType::P2WPKH => {
                AddressData::P2wpkh {
                    witness_program: self.witness_program.clone().unwrap_or(WitnessProgram {
                        version: 0,
                        program: vec![0u8; 20],
                    })
                }
            },
            AddressType::P2WSH => {
                AddressData::P2wsh {
                    witness_program: self.witness_program.clone().unwrap_or(WitnessProgram {
                        version: 0,
                        program: vec![0u8; 32],
                    })
                }
            },
        }
    }
    
    pub fn script_pubkey(&self) -> ScriptBuf {
        match self.address_type {
            AddressType::P2PKH => {
                // P2PKH script: OP_DUP OP_HASH160 <pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG
                let pubkey_hash = self.pubkey_hash.unwrap_or([0u8; 20]);
                let mut script = Vec::new();
                script.push(0x76); // OP_DUP
                script.push(0xa9); // OP_HASH160
                script.push(20);   // Push 20 bytes
                script.extend_from_slice(&pubkey_hash);
                script.push(0x88); // OP_EQUALVERIFY
                script.push(0xac); // OP_CHECKSIG
                ScriptBuf { inner: script }
            },
            AddressType::P2SH => {
                // P2SH script: OP_HASH160 <script_hash> OP_EQUAL
                let script_hash = self.pubkey_hash.unwrap_or([0u8; 20]);
                let mut script = Vec::new();
                script.push(0xa9); // OP_HASH160
                script.push(20);   // Push 20 bytes
                script.extend_from_slice(&script_hash);
                script.push(0x87); // OP_EQUAL
                ScriptBuf { inner: script }
            },
            AddressType::P2WPKH => {
                // P2WPKH script: OP_0 <20-byte-pubkey-hash>
                let pubkey_hash = self.pubkey_hash.unwrap_or([0u8; 20]);
                let mut script = Vec::new();
                script.push(0x00); // OP_0
                script.push(20);   // Push 20 bytes
                script.extend_from_slice(&pubkey_hash);
                ScriptBuf { inner: script }
            },
            AddressType::P2WSH => {
                // P2WSH script: OP_0 <32-byte-script-hash>
                let script_hash = if let Some(witness_program) = &self.witness_program {
                    if witness_program.program.len() == 32 {
                        let mut hash = [0u8; 32];
                        hash.copy_from_slice(&witness_program.program);
                        hash
                    } else {
                        [0u8; 32]
                    }
                } else {
                    [0u8; 32]
                };
                let mut script = Vec::new();
                script.push(0x00); // OP_0
                script.push(32);   // Push 32 bytes
                script.extend_from_slice(&script_hash);
                ScriptBuf { inner: script }
            },
        }
    }
}

/// Implementation of address parsing from string format.
/// 
/// This implementation supports parsing the two most common Bitcoin address formats
/// with full validation including checksum verification.
impl std::str::FromStr for Address {
    type Err = AddressError;
    
    /// Parses a Bitcoin address string into an `Address` structure.
    /// 
    /// This method performs comprehensive validation including:
    /// - Format detection (P2PKH, P2SH, P2WPKH, P2WSH)
    /// - Encoding validation (Base58Check vs Bech32)
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
        // These are legacy Bitcoin addresses starting with '1' on mainnet
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
            let payload = &decoded[..21];        // version + pubkey_hash
            let checksum = &decoded[21..25];     // provided checksum
            let computed_checksum = double_sha256(payload);
            if &computed_checksum[..4] != checksum {
                return Err(AddressError::InvalidBase58);
            }
            
            Ok(Address {
                inner: s.to_string(),
                address_type: AddressType::P2PKH,
                pubkey_hash: Some(pubkey_hash),
                witness_program: None,
            })
        }
        // P2SH (Pay-to-Script-Hash) address parsing
        // These are legacy Bitcoin script addresses starting with '3' on mainnet
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
            let payload = &decoded[..21];        // version + script_hash
            let checksum = &decoded[21..25];     // provided checksum
            let computed_checksum = double_sha256(payload);
            if &computed_checksum[..4] != checksum {
                return Err(AddressError::InvalidBase58);
            }
            
            Ok(Address {
                inner: s.to_string(),
                address_type: AddressType::P2SH,
                pubkey_hash: Some(script_hash), // Store script hash in pubkey_hash field
                witness_program: None,
            })
        }
        // P2WPKH/P2WSH (Pay-to-Witness-Public-Key-Hash/Script-Hash) address parsing
        // These are segwit addresses starting with 'bc1' on mainnet
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
                        inner: s.to_string(),
                        address_type: AddressType::P2WPKH,
                        pubkey_hash: Some(pubkey_hash),
                        witness_program: Some(WitnessProgram {
                            version: witness_version,
                            program: witness_program,
                        }),
                    })
                },
                32 => {
                    // P2WSH: 32-byte script hash
                    Ok(Self {
                        inner: s.to_string(),
                        address_type: AddressType::P2WSH,
                        pubkey_hash: None, // P2WSH doesn't have a pubkey hash
                        witness_program: Some(WitnessProgram {
                            version: witness_version,
                            program: witness_program,
                        }),
                    })
                },
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
    /// Invalid Base58Check encoding (for P2PKH addresses).
    /// 
    /// This includes:
    /// - Invalid characters in the Base58 alphabet
    /// - Checksum validation failures
    /// - Invalid version bytes
    InvalidBase58,
    
    /// Invalid address length (typically for P2PKH addresses).
    /// 
    /// P2PKH addresses must be exactly 25 bytes when decoded:
    /// 1 byte version + 20 bytes pubkey_hash + 4 bytes checksum
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
}

impl std::fmt::Display for AddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AddressError::InvalidBase58 => write!(f, "Invalid base58 encoding"),
            AddressError::InvalidLength => write!(f, "Invalid address length"),
            AddressError::InvalidWitnessProgram => write!(f, "Invalid witness program"),
            AddressError::UnsupportedFormat => write!(f, "Unsupported address format"),
            AddressError::InvalidBech32 => write!(f, "Invalid bech32 encoding"),
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
/// 4. Convert witness version and program from 5-bit to 8-bit encoding
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
/// Returns `AddressError::InvalidBech32` for any decoding failures including:
/// - Invalid characters in the address
/// - Checksum validation failures  
/// - Invalid witness version or program length
/// - Non-mainnet HRP (not "bc")
fn decode_bech32(addr: &str) -> Result<(u8, Vec<u8>), AddressError> {
    // Parse the segwit address using the bech32 crate's segwit module
    // This handles the complete segwit address decoding including checksum validation
    let (hrp, witness_version, witness_program) = segwit::decode(addr)
        .map_err(|_| AddressError::InvalidBech32)?;
    
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
        },
        1..=16 => {
            // Future segwit versions: program must be 2-40 bytes per BIP-141
            if witness_program.len() < 2 || witness_program.len() > 40 {
                return Err(AddressError::InvalidWitnessProgram);
            }
        },
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
    pub fn all_zeros() -> Self {
        Self([0u8; 32])
    }
    
    pub fn from_byte_array(bytes: [u8; 32]) -> Self {
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
    pub fn new(txid: Txid, vout: u32) -> Self {
        Self { txid, vout }
    }
}

/// Transaction input
#[derive(Debug, Clone)]
pub struct TxIn {
    pub previous_output: OutPoint,
    pub script_sig: ScriptBuf,
    pub sequence: Sequence,
    pub witness: Witness,
}

/// Transaction output  
#[derive(Debug, Clone)]
pub struct TxOut {
    pub value: Amount,
    pub script_pubkey: ScriptBuf,
}

/// Transaction
#[derive(Debug, Clone)]
pub struct Transaction {
    pub version: Version,
    pub lock_time: LockTime,
    pub input: Vec<TxIn>,
    pub output: Vec<TxOut>,
}

/// Amount (simplified)
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
        
        // Version (4 bytes, little-endian)
        len += writer.write(&self.version.0.to_le_bytes())?;
        
        // Input count (compact size)
        len += write_compact_size(writer, self.input.len() as u64)?;
        
        // Inputs
        for input in &self.input {
            // Previous output (36 bytes)
            len += writer.write(&input.previous_output.txid.0)?;
            len += writer.write(&input.previous_output.vout.to_le_bytes())?;
            
            // Script sig
            len += write_compact_size(writer, input.script_sig.inner.len() as u64)?;
            len += writer.write(&input.script_sig.inner)?;
            
            // Sequence (4 bytes)
            len += writer.write(&input.sequence.0.to_le_bytes())?;
        }
        
        // Output count
        len += write_compact_size(writer, self.output.len() as u64)?;
        
        // Outputs
        for output in &self.output {
            // Value (8 bytes, little-endian)
            len += writer.write(&output.value.0.to_le_bytes())?;
            
            // Script pubkey
            len += write_compact_size(writer, output.script_pubkey.inner.len() as u64)?;
            len += writer.write(&output.script_pubkey.inner)?;
        }
        
        // Lock time (4 bytes)
        len += writer.write(&self.lock_time.0.to_le_bytes())?;
        
        Ok(len)
    }
}

fn write_compact_size<W: std::io::Write>(writer: &mut W, n: u64) -> Result<usize, std::io::Error> {
    if n < 0xfd {
        writer.write_all(&[n as u8])?;
        Ok(1)
    } else if n <= 0xffff {
        writer.write_all(&[0xfd])?;
        writer.write_all(&(n as u16).to_le_bytes())?;
        Ok(3)
    } else if n <= 0xffffffff {
        writer.write_all(&[0xfe])?;
        writer.write_all(&(n as u32).to_le_bytes())?;
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
    
    pub fn push_opcode(mut self, opcode: u8) -> Self {
        self.inner.push(opcode);
        self
    }
    
    pub fn push_slice(mut self, data: &[u8]) -> Self {
        if data.len() <= 75 {
            self.inner.push(data.len() as u8);
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
pub const OP_RETURN: u8 = 0x6a;

// Signature hash cache (simplified)
pub struct SighashCache {
    tx: Transaction,
}

impl SighashCache {
    pub fn new(tx: Transaction) -> Self {
        Self { tx }
    }
    
    pub fn segwit_v0_encode_signing_data_to<W: std::io::Write>(
        &mut self,
        writer: &mut W,
        input_index: usize,
        script_code: &ScriptBuf,
        value: Amount,
        sighash_type: EcdsaSighashType,
    ) -> Result<(), std::io::Error> {
        // Simplified segwit v0 sighash implementation
        // This is a placeholder - full implementation would be more complex
        
        // For MVP, just write some basic transaction data
        writer.write_all(&self.tx.version.0.to_le_bytes())?;
        writer.write_all(&[input_index as u8])?;
        writer.write_all(&script_code.inner)?;
        writer.write_all(&value.0.to_le_bytes())?;
        writer.write_all(&[sighash_type as u8])?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EcdsaSighashType {
    All = 0x01,
}