use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use thiserror::Error;
use tower::Service;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use defuse_outlayer_host::HostFunctions;
use defuse_outlayer_vm_runner::{self as vm_runner, VmRuntime};

use crate::types::{
    AccountId, ExecutionMetrics, ExecutionRequest, ExecutionResponse, ProjectEnv, ProjectStorage,
};

#[derive(Debug, Clone)]
pub struct WasmExecutorConfig {
    pub stdout_limit: usize,
    pub stderr_limit: usize,
}

impl Default for WasmExecutorConfig {
    fn default() -> Self {
        Self {
            stdout_limit: 4 * 1024 * 1024, // 4 MiB
            stderr_limit: 64 * 1024,       // 64 KiB
        }
    }
}

#[derive(Clone)]
pub struct WasmExecutionRequest {
    pub request: ExecutionRequest,
    pub component: wasmtime::component::Component,
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
    pub const fn new(runtime: Arc<VmRuntime<H>>, config: WasmExecutorConfig, host_template: H) -> Self {
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

impl<H: HostFunctions + Clone + Send + 'static> Service<WasmExecutionRequest>
    for WasmExecutor<H>
{
    type Response = ExecutionResponse;
    type Error = WasmEnvironmentInternalError;
    type Future = BoxFuture<'static, Result<ExecutionResponse, WasmEnvironmentInternalError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: WasmExecutionRequest) -> Self::Future {
        let runtime = Arc::clone(&self.runtime);
        let config = self.config.clone();
        let host_state = self.host_template.clone();
        Box::pin(async move {
            let stdout = MemoryOutputPipe::new(config.stdout_limit);
            let stderr = MemoryOutputPipe::new(config.stderr_limit);
            let ctx = vm_runner::Context::new(
                MemoryInputPipe::new(req.request.input),
                stdout.clone(),
                stderr.clone(),
                host_state,
            );

            let outcome = runtime
                .execute(ctx, &req.component)
                .await
                .map_err(WasmEnvironmentInternalError)?;
            let result = outcome
                .guest_error
                .map_or_else(|| Ok(stdout.contents()), |e| Err(anyhow::Error::from(e)));
            let instructions_used = outcome.fuel_consumed;

            Ok(ExecutionResponse {
                result,
                logs: stderr.contents(),
                metrics: ExecutionMetrics { instructions_used },
                storage: ProjectStorage,
            })
        })
    }
}
