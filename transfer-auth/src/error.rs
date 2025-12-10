use near_sdk::{FunctionError, PromiseError};

use thiserror::Error as ThisError;

#[derive(Debug, ThisError, FunctionError)]
pub enum Error {
    #[error("borsh: {0}")]
    Borsh(near_sdk::borsh::io::Error),
    #[error("ContractIsBeingDestroyed")]
    ContractIsBeingDestroyed,
    #[error("Timeout or promise error {0:?}")]
    PromiseError(PromiseError),
}

impl From<PromiseError> for Error {
    fn from(e: PromiseError) -> Self {
        Self::PromiseError(e)
    }
}
