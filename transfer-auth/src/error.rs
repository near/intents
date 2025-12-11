use near_sdk::FunctionError;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError, FunctionError)]
pub enum Error {
    #[error("borsh: {0}")]
    Borsh(near_sdk::borsh::io::Error),
}

