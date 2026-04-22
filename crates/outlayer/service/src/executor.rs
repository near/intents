use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use thiserror::Error;
use tower::Service;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use defuse_outlayer_host_functions::HostFunctions;
use defuse_outlayer_vm_runner::{self as vm_runner, VmRuntime};

use crate::types::{
    ExecutionMetrics, ExecutionResponse, OnChainDestination, ProjectStorage, WasmExecutionRequest,
};

const STDOUT_LIMIT: usize = 4 * 1024 * 1024; // 4 MiB
const STDERR_LIMIT: usize = 64 * 1024;       // 64 KiB

#[derive(Debug, Error)]
pub enum WasmExecutorError {
    #[error(transparent)]
    Execution(#[from] vm_runner::ExecutionError),
}

pub struct WasmExecutor<H: HostFunctions + Default + 'static> {
    runtime: Arc<VmRuntime<H>>,
}

impl<H: HostFunctions + Default + 'static> WasmExecutor<H> {
    pub const fn new(runtime: Arc<VmRuntime<H>>) -> Self {
        Self { runtime }
    }
}

impl<H: HostFunctions + Default + 'static> Clone for WasmExecutor<H> {
    fn clone(&self) -> Self {
        Self { runtime: Arc::clone(&self.runtime) }
    }
}

impl<H: HostFunctions + Default + Send + 'static> Service<WasmExecutionRequest>
    for WasmExecutor<H>
{
    type Response = ExecutionResponse;
    type Error = WasmExecutorError;
    type Future = BoxFuture<'static, Result<ExecutionResponse, WasmExecutorError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), WasmExecutorError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: WasmExecutionRequest) -> Self::Future {
        let runtime = Arc::clone(&self.runtime);
        Box::pin(async move {
            let stdout = MemoryOutputPipe::new(STDOUT_LIMIT);
            let stderr = MemoryOutputPipe::new(STDERR_LIMIT);
            let ctx = vm_runner::Context::new(
                MemoryInputPipe::new(req.request.input),
                stdout.clone(),
                stderr.clone(),
                H::default(),
            );

            let outcome = runtime.execute(ctx, &req.component).await?;

            Ok(ExecutionResponse {
                request_id: req.request.id,
                respond_to: OnChainDestination { contract_id: req.request.project_id },
                result: Ok(stdout.contents()),
                metrics: ExecutionMetrics {
                    instructions_used: outcome.fuel_consumed,
                    ..ExecutionMetrics::default()
                },
                storage: ProjectStorage,
            })
        })
    }
}
