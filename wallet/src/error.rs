use near_sdk::{AccountId, FunctionError, borsh};
use thiserror::Error as ThisError;

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, ThisError, FunctionError)]
pub enum Error {
    #[error("borsh: {0}")]
    Borsh(#[from] borsh::io::Error),

    #[error("signature expired")]
    Expired,

    #[error("extension '{0}' already exists")]
    ExtensionExists(AccountId),

    #[error("extension '{0}' is not enabled")]
    ExtensionNotExist(AccountId),

    #[error("invalid chain_id: '{got}', expected: {expected}")]
    InvalidChainId { got: String, expected: String },

    #[error("invalid seqno: {got}, expected: {expected}")]
    InvalidSeqno { got: u32, expected: u32 },

    #[error("invalid signature")]
    InvalidSignature,

    #[error("invalid signer_id: {0}")]
    InvalidSignerId(AccountId),

    #[error("insufficient attached deposit")]
    InsufficientDeposit,

    #[error("lockout: signature is disabled and extensions are empty")]
    Lockout,

    #[error("signature is disabled")]
    SignatureDisabled,

    #[error("this signature mode is already set")]
    ThisSignatureModeAlreadySet,
}
