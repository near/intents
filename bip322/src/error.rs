//! Error types for BIP-322 signature verification
//!
//! This module contains error types for address parsing and other operations
//! in the BIP-322 implementation.

/// Address parsing error type.
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

    UnsupportedWitnessVersion,

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
            Self::InvalidBase58 => write!(f, "Invalid base58 encoding"),
            Self::InvalidLength => write!(f, "Invalid address length"),
            Self::InvalidWitnessProgram => write!(f, "Invalid witness program"),
            Self::UnsupportedFormat => write!(f, "Unsupported address format"),
            Self::UnsupportedWitnessVersion => write!(f, "Unsupported witness version"),
            Self::InvalidBech32 => write!(f, "Invalid bech32 encoding"),
        }
    }
}

impl std::error::Error for AddressError {}
