use thiserror::Error;

use crate::{
    env_fetch::EnvFetchError, executor::WasmEnvironmentInternalError, resolver::ResolveError,
    storage_fetch::StorageFetchError,
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
}
