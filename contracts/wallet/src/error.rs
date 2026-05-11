use defuse_wallet_state::NoncesError;
use near_sdk::{AccountId, FunctionError};
use thiserror::Error as ThisError;

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, ThisError, FunctionError)]
pub enum Error {
    #[error("already executed")]
    AlreadyExecuted,

    #[error("extension '{0}' is already enabled")]
    ExtensionEnabled(AccountId),

    #[error("extension '{0}' is not enabled")]
    ExtensionNotEnabled(AccountId),

    #[error("invalid chain_id")]
    InvalidChainId,

    #[error("expired or from the future")]
    ExpiredOrFuture,

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

impl From<NoncesError> for Error {
    fn from(e: NoncesError) -> Self {
        match e {
            NoncesError::AlreadyExecuted => Self::AlreadyExecuted,
            NoncesError::ExpiredOrFuture => Self::ExpiredOrFuture,
        }
    }
}
