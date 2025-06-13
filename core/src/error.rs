use crate::{
    engine::deltas::InvariantViolated,
    token_id::{TokenId, error::TokenIdError, nep171::Nep171TokenId},
};
use near_sdk::{FunctionError, serde_json};
use thiserror::Error as ThisError;

pub type Result<T, E = DefuseError> = ::core::result::Result<T, E>;

#[derive(Debug, ThisError, FunctionError)]
pub enum DefuseError {
    #[error("account not found")]
    AccountNotFound,

    #[error("insufficient balance or overflow")]
    BalanceOverflow,

    #[error("deadline has expired")]
    DeadlineExpired,

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

    #[error("public key already exists")]
    PublicKeyExists,

    #[error("public key doesn't exist")]
    PublicKeyNotExist,

    #[error("token_id: {0}")]
    ParseTokenId(#[from] TokenIdError),

    #[error("wrong verifying_contract")]
    WrongVerifyingContract,
}
