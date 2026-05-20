use thiserror::Error as ThisError;

#[derive(Debug, ThisError, PartialEq, Eq)]
pub enum ParseCurveError {
    #[error("wrong curve type")]
    WrongCurveType,
    #[error("base58: {0}")]
    #[cfg(feature = "parse")]
    Base58(#[from] bs58::decode::Error),
    #[error("invalid length")]
    InvalidLength,
}
