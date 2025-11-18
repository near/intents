use near_sdk::{FunctionError, serde_json};
use thiserror::Error as ThisError;

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, ThisError, FunctionError)]
pub enum Error {
    #[error("cleanup in progress")]
    CleanupInProgress,
    #[error("closed")]
    Closed,
    #[error("deadline has expired")]
    DeadlineExpired,
    #[error("excessive fees")]
    ExcessiveFees,
    #[error("impossible to fill: required gas is too big")]
    ExcessiveGas,
    #[error("integer overflow")]
    IntegerOverflow,
    #[error("insufficient amount")]
    InsufficientAmount,
    #[error("invalid data")]
    InvalidData,
    #[error("JSON: {0}")]
    JSON(#[from] serde_json::Error),
    #[error("partial fills are not allowed")]
    PartialFillsNotAllowed,
    #[error("price is too low")]
    PriceTooLow,
    #[error("same tokens")]
    SameTokens,
    #[error("unauthorized")]
    Unauthorized,
    #[error("wrong token")]
    WrongToken,
}
