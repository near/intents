use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::future::BoxFuture;
use thiserror::Error;
use tower::Service;

use crate::types::{
    ExecutionMetrics, ExecutionResponse, OnChainDestination, ProjectStorage, WasmExecutionRequest,
};

#[derive(Debug, Error)]
pub enum WasmExecutorError {
    #[error("not implemented")]
    NotImplemented,
}

/// Innermost Tower service. Executes the WASM module and captures stdout.
///
/// `Error = Infallible`: all WASM-level failures (traps, resource limits,
/// compilation) are represented in `ExecutionResponse::output: Err(ExecutionError)`.
#[derive(Clone)]
pub struct WasmExecutor;

impl WasmExecutor {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for WasmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Service<WasmExecutionRequest> for WasmExecutor {
    type Response = ExecutionResponse;
    type Error = WasmExecutorError;
    type Future = BoxFuture<'static, Result<ExecutionResponse, WasmExecutorError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), WasmExecutorError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: WasmExecutionRequest) -> Self::Future {
        Box::pin(async move {
            Ok(ExecutionResponse {
                respond_to: OnChainDestination { contract_id: req.request.project_id.clone() },
                request: req.request,
                output: Ok(Bytes::new()),
                metrics: ExecutionMetrics::default(),
                storage: ProjectStorage,
            })
        })
    }
}
