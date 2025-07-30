//! Error types for BIP-322 signature verification
//!
//! This module contains comprehensive error types for all failure modes
//! in BIP-322 signature verification, providing detailed context for
//! debugging and integration purposes.

use crate::bitcoin_minimal::AddressType;

/// Main error type for BIP-322 operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bip322Error {
    /// Errors related to witness stack format and content
    Witness(WitnessError),

    /// Errors in signature parsing and validation
    Signature(SignatureError),

    /// Errors in script execution and validation
    Script(ScriptError),

    /// Errors in cryptographic operations
    Crypto(CryptoError),

    /// Errors in address validation and derivation
    Address(AddressValidationError),

    /// Errors in BIP-322 transaction construction
    Transaction(TransactionError),
}

/// Errors related to witness stack format and content
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessError {
    /// Witness stack is empty when signature data is expected
    EmptyWitness,

    /// Insufficient witness stack elements for the address type
    /// Contains: (`expected_count`, `actual_count`)
    InsufficientElements(usize, usize),

    /// Invalid witness stack element at specified index
    /// Contains: (`element_index`, description)
    InvalidElement(usize, String),

    /// Witness stack format doesn't match address type requirements
    /// Contains: (`address_type`, description)
    FormatMismatch(AddressType, String),
}

/// Errors in signature parsing and validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureError {
    /// Signature components (r, s) are invalid
    /// Contains: description of the invalid component
    InvalidComponents(String),

    /// Recovery ID could not be determined
    /// All recovery IDs (0-3) failed during signature recovery
    RecoveryIdNotFound,

    /// Signature recovery failed with the determined recovery ID
    /// Contains: (`recovery_id`, description)
    RecoveryFailed(u8, String),

    /// Public key recovered from signature doesn't match provided public key
    /// Contains: (`expected_pubkey_hex`, `recovered_pubkey_hex`)
    PublicKeyMismatch(String, String),
}

/// Errors in script execution and validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    /// Script hash doesn't match the address
    /// Contains: (`expected_hash_hex`, `computed_hash_hex`)
    HashMismatch(String, String),

    /// Script format is not supported
    /// Contains: (`script_hex`, reason)
    UnsupportedFormat(String, String),

    /// Script execution failed during validation
    /// Contains: (operation, reason)
    ExecutionFailed(String, String),

    /// Script size exceeds limits
    /// Contains: (`actual_size`, `max_size`)
    SizeExceeded(usize, usize),

    /// Invalid opcode or script structure
    /// Contains: (position, opcode, description)
    InvalidOpcode(usize, u8, String),

    /// Public key in script doesn't match provided public key
    /// Contains: (`script_pubkey_hash_hex`, `computed_pubkey_hash_hex`)
    PubkeyMismatch(String, String),
}

/// Errors in cryptographic operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// ECDSA signature recovery failed
    /// Contains: description of the failure
    EcrecoverFailed(String),

    /// Public key format is invalid
    /// Contains: (`pubkey_hex`, reason)
    InvalidPublicKey(String, String),

    /// Hash computation failed
    /// Contains: (`hash_type`, reason)
    HashingFailed(String, String),

    /// NEAR SDK cryptographic function failed
    /// Contains: (`function_name`, description)
    NearSdkError(String, String),
}

/// Errors in address validation and derivation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressValidationError {
    /// Address type doesn't support the requested operation
    /// Contains: (`address_type`, operation)
    UnsupportedOperation(AddressType, String),

    /// Public key doesn't derive to the claimed address
    /// Contains: (`claimed_address`, `derived_address`)
    DerivationMismatch(String, String),

    /// Address parsing or validation failed
    /// Contains: (address, reason)
    InvalidAddress(String, String),

    /// Missing required address data (`pubkey_hash`, `witness_program`, etc.)
    /// Contains: (`address_type`, `missing_field`)
    MissingData(AddressType, String),
}

/// Errors in BIP-322 transaction construction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    /// Failed to create the "`to_spend`" transaction
    /// Contains: reason for failure
    ToSpendCreationFailed(String),

    /// Failed to create the "`to_sign`" transaction
    /// Contains: reason for failure
    ToSignCreationFailed(String),

    /// Message hash computation failed
    /// Contains: (stage, reason)
    MessageHashFailed(String, String),

    /// Transaction encoding failed
    /// Contains: (`transaction_type`, reason)
    EncodingFailed(String, String),
}

impl std::fmt::Display for Bip322Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Witness(e) => write!(f, "Witness error: {e}"),
            Self::Signature(e) => write!(f, "Signature error: {e}"),
            Self::Script(e) => write!(f, "Script error: {e}"),
            Self::Crypto(e) => write!(f, "Crypto error: {e}"),
            Self::Address(e) => write!(f, "Address error: {e}"),
            Self::Transaction(e) => write!(f, "Transaction error: {e}"),
        }
    }
}

impl std::fmt::Display for WitnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyWitness => write!(f, "Witness stack is empty"),
            Self::InsufficientElements(expected, actual) => {
                write!(
                    f,
                    "Insufficient witness elements: expected {expected}, got {actual}"
                )
            }
            Self::InvalidElement(idx, desc) => {
                write!(f, "Invalid witness element at index {idx}: {desc}")
            }
            Self::FormatMismatch(addr_type, desc) => {
                write!(f, "Witness format mismatch for {addr_type:?}: {desc}")
            }
        }
    }
}

impl std::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidComponents(desc) => {
                write!(f, "Invalid signature components: {desc}")
            }
            Self::RecoveryIdNotFound => {
                write!(f, "Could not determine recovery ID (tried 0-3)")
            }
            Self::RecoveryFailed(id, desc) => {
                write!(f, "Signature recovery failed with ID {id}: {desc}")
            }
            Self::PublicKeyMismatch(expected, recovered) => {
                write!(
                    f,
                    "Public key mismatch: expected {expected}, recovered {recovered}"
                )
            }
        }
    }
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HashMismatch(expected, computed) => {
                write!(
                    f,
                    "Script hash mismatch: expected {expected}, computed {computed}"
                )
            }
            Self::UnsupportedFormat(script, reason) => {
                write!(f, "Unsupported script format {script}: {reason}")
            }
            Self::ExecutionFailed(op, reason) => {
                write!(f, "Script execution failed at {op}: {reason}")
            }
            Self::SizeExceeded(actual, max) => {
                write!(f, "Script size {actual} exceeds maximum {max}")
            }
            Self::InvalidOpcode(pos, opcode, desc) => {
                write!(f, "Invalid opcode 0x{opcode:02x} at position {pos}: {desc}")
            }
            Self::PubkeyMismatch(script_hash, computed_hash) => {
                write!(
                    f,
                    "Script pubkey mismatch: script has {script_hash}, computed {computed_hash}"
                )
            }
        }
    }
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EcrecoverFailed(desc) => {
                write!(f, "ECDSA signature recovery failed: {desc}")
            }
            Self::InvalidPublicKey(pubkey, reason) => {
                write!(f, "Invalid public key {pubkey}: {reason}")
            }
            Self::HashingFailed(hash_type, reason) => {
                write!(f, "{hash_type} hashing failed: {reason}")
            }
            Self::NearSdkError(func, desc) => {
                write!(f, "NEAR SDK {func} failed: {desc}")
            }
        }
    }
}

impl std::fmt::Display for AddressValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedOperation(addr_type, op) => {
                write!(f, "{addr_type:?} addresses don't support operation: {op}")
            }
            Self::DerivationMismatch(claimed, derived) => {
                write!(
                    f,
                    "Address derivation mismatch: claimed {claimed}, derived {derived}"
                )
            }
            Self::InvalidAddress(addr, reason) => {
                write!(f, "Invalid address {addr}: {reason}")
            }
            Self::MissingData(addr_type, field) => {
                write!(f, "{addr_type:?} address missing required data: {field}")
            }
        }
    }
}

impl std::fmt::Display for TransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToSpendCreationFailed(reason) => {
                write!(f, "Failed to create to_spend transaction: {reason}")
            }
            Self::ToSignCreationFailed(reason) => {
                write!(f, "Failed to create to_sign transaction: {reason}")
            }
            Self::MessageHashFailed(stage, reason) => {
                write!(f, "Message hash computation failed at {stage}: {reason}")
            }
            Self::EncodingFailed(tx_type, reason) => {
                write!(f, "Transaction encoding failed for {tx_type}: {reason}")
            }
        }
    }
}

impl std::error::Error for Bip322Error {}
impl std::error::Error for WitnessError {}
impl std::error::Error for SignatureError {}
impl std::error::Error for ScriptError {}
impl std::error::Error for CryptoError {}
impl std::error::Error for AddressValidationError {}
impl std::error::Error for TransactionError {}

/// Result type for BIP-322 operations
pub type Bip322Result<T> = Result<T, Bip322Error>;
