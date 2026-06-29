use near_sdk::{AccountId, FunctionError};
use thiserror::Error as ThisError;

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, ThisError, FunctionError)]
pub enum Error {
    #[error("extension '{0}' is already enabled")]
    ExtensionEnabled(AccountId),

    #[error("extension '{0}' is not enabled")]
    ExtensionNotEnabled(AccountId),

    #[error("invalid chain_id")]
    InvalidChainId,

    #[error("invalid signature")]
    InvalidSignature,

    #[error("invalid signer_id: {0}")]
    InvalidSignerId(AccountId),

    #[error("insufficient attached deposit")]
    InsufficientDeposit,

    #[error("lockout: signature is disabled and extensions are empty")]
    Lockout,

    #[error("self-calls are not allowed")]
    SelfCallsNotAllowed,

    #[error("signature is disabled")]
    SignatureDisabled,

    #[error("this signature mode is already set")]
    ThisSignatureModeAlreadySet,

    #[error("unsupported promise action")]
    UnsupportedPromiseAction,
}
