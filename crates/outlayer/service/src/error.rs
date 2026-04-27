use thiserror::Error;
use tower::BoxError;

use crate::{
    env::EnvFetchError, executor::WasmEnvironmentInternalError, resolver::ResolveError,
    storage::StorageFetchError,
};

/// Top-level error type for the composed Tower stack.
#[derive(Debug, Error)]
pub enum ExecutionStackError {
    #[error(transparent)]
    Fetch(#[from] ResolveError),
    #[error(transparent)]
    EnvFetch(#[from] EnvFetchError),
    #[error(transparent)]
    StorageFetch(#[from] StorageFetchError),
    #[error(transparent)]
    Executor(#[from] WasmEnvironmentInternalError),
    #[error("wasm compilation failed: {0}")]
    Compile(anyhow::Error),
    #[error("execution timed out")]
    Timeout,
    #[error("internal error: {0}")]
    Internal(BoxError),
}
