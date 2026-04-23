use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use thiserror::Error;
use tower::Service;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use defuse_outlayer_host_functions::HostFunctions;
use defuse_outlayer_vm_runner::{self as vm_runner, ExecutionError, VmRuntime};

use crate::types::{
    AccountId, ExecutionMetrics, ExecutionResponse, OnChainRequest, ProjectEnv, ProjectStorage,
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
    pub request: OnChainRequest,
    pub component: wasmtime::component::Component,
    pub storage: ProjectStorage,
    pub env: ProjectEnv,
    pub caller: Option<AccountId>,
}

/// Infrastructure-level failure: the VM itself couldn't run, not the WASM.
#[derive(Debug, Error)]
#[error("vm infrastructure error: {0}")]
pub struct WasmEnvironmentInternalError(#[from] anyhow::Error);

pub struct WasmExecutor<H: HostFunctions + Default + 'static> {
    runtime: Arc<VmRuntime<H>>,
    config: WasmExecutorConfig,
}

impl<H: HostFunctions + Default + 'static> WasmExecutor<H> {
    pub fn new(runtime: Arc<VmRuntime<H>>, config: WasmExecutorConfig) -> Self {
        Self { runtime, config }
    }
}

impl<H: HostFunctions + Default + 'static> Clone for WasmExecutor<H> {
    fn clone(&self) -> Self {
        Self {
            runtime: Arc::clone(&self.runtime),
            config: self.config.clone(),
        }
    }
}

impl<H: HostFunctions + Default + Send + 'static> Service<WasmExecutionRequest>
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
        Box::pin(async move {
            let stdout = MemoryOutputPipe::new(config.stdout_limit);
            let stderr = MemoryOutputPipe::new(config.stderr_limit);
            let ctx = vm_runner::Context::new(
                MemoryInputPipe::new(req.request.input),
                stdout.clone(),
                stderr.clone(),
                H::default(),
            );

            // TODO: vm-runner should ideally return `Result<Result<Outcome, WasmError>, VmError>` so
            // the separation between an execution-environment failure (linker, instantiation,
            // internal wasmtime error) and a WASM-level failure (trap, non-zero exit) is
            // expressed in the type rather than by matching on a single flat enum here.
            let (result, instructions_used) = match runtime.execute(ctx, &req.component).await {
                Ok(outcome) => (Ok(stdout.contents()), outcome.fuel_consumed),
                Err(ExecutionError::Unknown { source }) => {
                    return Err(WasmEnvironmentInternalError(source));
                }
                Err(e) => (Err(e.into()), 0),
            };

            Ok(ExecutionResponse {
                nonce: req.request.nonce,
                wasm_hash: req.request.wasm_hash,
                project_id: req.request.project_id,
                result,
                logs: stderr.contents(),
                metrics: ExecutionMetrics {
                    instructions_used,
                    ..ExecutionMetrics::default()
                },
                storage: ProjectStorage,
            })
        })
    }
}
