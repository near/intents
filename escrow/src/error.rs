use near_sdk::{FunctionError, serde_json};
use thiserror::Error as ThisError;

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, ThisError, FunctionError)]
pub enum Error {
    #[error("closed")]
    Closed,
    #[error("integer overflow")]
    IntegerOverflow,
    #[error("JSON: {0}")]
    JSON(#[from] serde_json::Error),
    #[error("can't set to lower price")]
    LowerPrice,
    #[error("partial fills are not allowed")]
    PartialFillsNotAllowed,
    #[error("unauthorized")]
    Unauthorized,
    #[error("wrong asset")]
    WrongAsset,
    #[error("wrong data")]
    WrongData,
}
