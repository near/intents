use near_account_id::AccountId;
use thiserror::Error as ThisError;

use crate::NonceError;

/// An error that can occur in [`Wallet`](crate::contract::Wallet) contract.
#[cfg_attr(feature = "near-contract", derive(::near_sdk::FunctionError))]
#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum ContractError {
    #[error("extension '{0}' is already enabled")]
    ExtensionEnabled(AccountId),

    #[error("extension '{0}' is not enabled")]
    ExtensionNotEnabled(AccountId),

    #[error("insufficient attached deposit")]
    InsufficientDeposit,

    #[error("invalid chain_id")]
    InvalidChainId,

    #[error("invalid path")]
    InvalidPath,

    #[error("invalid signature")]
    InvalidSignature,

    #[error("invalid signer_id: {0}")]
    InvalidSignerId(AccountId),

    #[cfg(feature = "json")]
    #[error("JSON: {0}")]
    JSON(#[from] serde_json::Error),

    #[error("lockout: signature is disabled and extensions are empty")]
    Lockout,

    #[error("nonce: {0}")]
    Nonce(#[from] NonceError),

    #[error("self-calls are not allowed")]
    SelfCallsNotAllowed,

    #[error("signature is disabled, use extensions to act on behelf of this wallet")]
    SignatureDisabled,

    #[error("this signature mode is already set")]
    ThisSignatureModeAlreadySet,

    #[error("unsupported promise action")]
    UnsupportedPromiseAction,
}
