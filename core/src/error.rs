use crate::{
    engine::deltas::InvariantViolated,
    token_id::{TokenId, error::TokenIdError, nep171::Nep171TokenId},
};
use defuse_crypto::PublicKey;
use near_sdk::{AccountId, FunctionError, serde_json};
use thiserror::Error as ThisError;

pub type Result<T, E = DefuseError> = ::core::result::Result<T, E>;

#[derive(Debug, ThisError, FunctionError)]
pub enum DefuseError {
    #[error("account '{0}' not found")]
    AccountNotFound(AccountId),

    #[error("account '{0}' is locked")]
    AccountLocked(AccountId),

    #[error("authentication by PREDECESSOR_ID is disabled for account '{0}'")]
    AuthByPredecessorIdDisabled(AccountId),

    #[error("insufficient balance or overflow")]
    BalanceOverflow,

    #[error("deadline has expired")]
    DeadlineExpired,

    #[error("deadline is greater than nonce")]
    DeadlineGreaterThanNonce,

    #[error("gas overflow")]
    GasOverflow,

    #[error("invalid intent")]
    InvalidIntent,

    #[error("invalid signature")]
    InvalidSignature,

    #[error(
        "invariant violated: {}",
        serde_json::to_string(.0).unwrap_or_else(|_| unreachable!())
    )]
    InvariantViolated(InvariantViolated),

    #[error("JSON: {0}")]
    JSON(#[from] serde_json::Error),

    #[error("NFT '{}' is already deposited", TokenId::Nep171(.0.clone()))]
    NftAlreadyDeposited(Nep171TokenId),

    #[error("nonce was already used")]
    NonceUsed,

    #[error("nonce was already expired")]
    NonceExpired,

    #[error("public key '{1}' already exists for account '{0}'")]
    PublicKeyExists(AccountId, PublicKey),

    #[error("public key '{1}' doesn't exist for account '{0}'")]
    PublicKeyNotExist(AccountId, PublicKey),

    #[error("token_id: {0}")]
    ParseTokenId(#[from] TokenIdError),

    #[error("wrong verifying_contract")]
    WrongVerifyingContract,
}
