use std::sync::Arc;
use std::task::{Context, Poll};

use defuse_outlayer_vm_runner::HostFunctions;
use defuse_outlayer_vm_runner::{
    self as vm_runner, Component, MemoryInputPipe, MemoryOutputPipe, VmRuntime,
};
use futures_util::future::BoxFuture;
use thiserror::Error;
use tower::Service;
use tracing::Instrument as _;

use crate::types::{
    AccountId, ExecutionMetrics, ExecutionRequest, ExecutionResponse, ProjectEnv, ProjectStorage,
};

#[derive(Debug, Clone)]
pub struct WasmExecutorConfig {
    pub stdout_limit: usize,
    pub stderr_limit: usize,
    pub fuel_limit: u64,
}

impl Default for WasmExecutorConfig {
    fn default() -> Self {
        Self {
            stdout_limit: 4 * 1024 * 1024, // 4 MiB
            stderr_limit: 64 * 1024,       // 64 KiB
            fuel_limit: 1_000_000_000,
        }
    }
}

#[derive(Clone)]
pub struct WasmExecutionRequest {
    pub request: ExecutionRequest,
    pub component: Component,
    pub storage: ProjectStorage,
    pub env: ProjectEnv,
    pub caller: Option<AccountId>,
}

/// Infrastructure-level failure: the VM itself couldn't run, not the WASM.
#[derive(Debug, Error)]
#[error("vm infrastructure error: {0}")]
pub struct WasmEnvironmentInternalError(#[from] anyhow::Error);

pub struct WasmExecutor<H: HostFunctions + Clone + 'static> {
    runtime: Arc<VmRuntime<H>>,
    config: WasmExecutorConfig,
    host_template: H,
}

impl<H: HostFunctions + Clone + 'static> WasmExecutor<H> {
    pub const fn new(
        runtime: Arc<VmRuntime<H>>,
        config: WasmExecutorConfig,
        host_template: H,
    ) -> Self {
        Self {
            runtime,
            config,
            host_template,
        }
    }
}

impl<H: HostFunctions + Clone + 'static> Clone for WasmExecutor<H> {
    fn clone(&self) -> Self {
        Self {
            runtime: Arc::clone(&self.runtime),
            config: self.config.clone(),
            host_template: self.host_template.clone(),
        }
    }
}

impl<H: HostFunctions + Clone + Send + 'static> Service<WasmExecutionRequest> for WasmExecutor<H> {
    type Response = ExecutionResponse;
    type Error = WasmEnvironmentInternalError;
    type Future = BoxFuture<'static, Result<ExecutionResponse, WasmEnvironmentInternalError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[tracing::instrument(level = "debug", name = "wasm.execute", skip_all)]
    fn call(&mut self, req: WasmExecutionRequest) -> Self::Future {
        let runtime = Arc::clone(&self.runtime);
        let config = self.config.clone();
        let host_state = self.host_template.clone();
        Box::pin(
            async move {
                let stdout = MemoryOutputPipe::new(config.stdout_limit);
                let stderr = MemoryOutputPipe::new(config.stderr_limit);
                let ctx = vm_runner::Context::new(
                    MemoryInputPipe::new(req.request.input),
                    stdout.clone(),
                    stderr.clone(),
                    host_state,
                )
                .fuel_limit(config.fuel_limit);

                let outcome = runtime
                    .execute(ctx, &req.component)
                    .await
                    .map_err(WasmEnvironmentInternalError)?;
                let instructions_used = outcome.details.fuel_consumed;
                let stdout_bytes = stdout.contents();
                let stderr_bytes = stderr.contents();
                tracing::debug!(
                    instructions_used,
                    stdout_len = stdout_bytes.len(),
                    stderr_len = stderr_bytes.len(),
                    "wasm execution finished"
                );
                if outcome.error.is_some() {
                    tracing::warn!(instructions_used, "wasm component returned error");
                }
                let result = outcome
                    .error
                    .map_or_else(|| Ok(stdout_bytes), |e| Err(anyhow::Error::from(e)));

                Ok(ExecutionResponse {
                    result,
                    logs: stderr_bytes,
                    metrics: ExecutionMetrics { instructions_used },
                    storage: ProjectStorage,
                })
            }
            .instrument(tracing::Span::current()),
        )
    }
}
