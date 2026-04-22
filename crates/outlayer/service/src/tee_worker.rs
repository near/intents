use std::task::{ready, Context, Poll};

use futures_util::future::BoxFuture;
use tower::Service;
use wasmtime::component::Component;

use crate::{
    error::ExecutionStackError,
    executor,
    types::{
        AccountId, ExecutionResponse, ProjectEnv,
        ProjectStorage, Request, ResourceLimits, WasmExecutionRequest,
    },
};

pub struct TeeWorkerService<E, W, Env, St> {
    executor: E,
    wasm:     W,
    env:      Env,
    storage:  St,
}

impl<E, W, Env, St> TeeWorkerService<E, W, Env, St> {
    pub const fn new(executor: E, wasm: W, env: Env, storage: St) -> Self {
        Self { executor, wasm, env, storage }
    }
}

impl<E, W, Env, St> Service<Request> for TeeWorkerService<E, W, Env, St>
where
    E: Service<WasmExecutionRequest, Response = ExecutionResponse, Error = executor::WasmExecutorError>
        + Send
        + Clone
        + 'static,
    E::Future: Send,
    W: Service<(String, [u8; 32]), Response = Component, Error = ExecutionStackError>
        + Send
        + Clone
        + 'static,
    W::Future: Send,
    Env: Service<AccountId, Response = ProjectEnv, Error = ExecutionStackError>
        + Send
        + Clone
        + 'static,
    Env::Future: Send,
    St: Service<AccountId, Response = ProjectStorage, Error = ExecutionStackError>
        + Send
        + Clone
        + 'static,
    St::Future: Send,
{
    type Response = ExecutionResponse;
    type Error = ExecutionStackError;
    type Future = BoxFuture<'static, Result<ExecutionResponse, ExecutionStackError>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ExecutionStackError>> {
        ready!(self.wasm.poll_ready(cx))?;
        ready!(self.env.poll_ready(cx))?;
        ready!(self.storage.poll_ready(cx))?;
        ready!(self.executor.poll_ready(cx)).map_err(ExecutionStackError::from)?;
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut executor = self.executor.clone();
        let mut wasm    = self.wasm.clone();
        let mut env     = self.env.clone();
        let mut storage = self.storage.clone();

        Box::pin(async move {
            let Request::OnChain(onchain) = req else {
                todo!("offchain not yet implemented")
            };

            let (component_res, env_res, storage_res) = tokio::join!(
                wasm.call((onchain.wasm_url.clone(), onchain.wasm_hash)),
                env.call(onchain.project_id.clone()),
                storage.call(onchain.project_id.clone()),
            );

            let wasm_req = WasmExecutionRequest {
                component: component_res?,
                request: onchain,
                env: env_res?,
                storage: storage_res?,
                caller: None,
                limits: ResourceLimits,
            };

            let response = executor.call(wasm_req).await?;
            Ok(response)
        })
    }
}
