use std::task::{Context, Poll, ready};

use futures_util::future::BoxFuture;
use tower::Service;
use wasmtime::component::Component;

use crate::{
    error::ExecutionStackError,
    executor::{self, WasmExecutionRequest},
    types::{AccountId, ExecutionRequest, ExecutionResponse, ProjectEnv, ProjectStorage},
};

#[derive(Clone)]
pub struct OutlayerService<E, W, Env, St> {
    executor: E,
    wasm: W,
    env: Env,
    storage: St,
}

impl<E, W, Env, St> OutlayerService<E, W, Env, St> {
    pub const fn new(executor: E, wasm: W, env: Env, storage: St) -> Self {
        Self {
            executor,
            wasm,
            env,
            storage,
        }
    }
}

impl<E, W, Env, St> Service<ExecutionRequest> for OutlayerService<E, W, Env, St>
where
    E: Service<
            WasmExecutionRequest,
            Response = ExecutionResponse,
            Error = executor::WasmEnvironmentInternalError,
        > + Send
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

    fn call(&mut self, req: ExecutionRequest) -> Self::Future {
        let mut executor = self.executor.clone();
        let mut wasm = self.wasm.clone();
        let mut env = self.env.clone();
        let mut storage = self.storage.clone();

        Box::pin(async move {
            let (component_res, env_res, storage_res) = tokio::join!(
                wasm.call((req.wasm_url.clone(), req.wasm_hash)),
                env.call(req.project_id.clone()),
                storage.call(req.project_id.clone()),
            );

            let wasm_req = WasmExecutionRequest {
                component: component_res?,
                request: req,
                env: env_res?,
                storage: storage_res?,
                caller: None,
            };

            Ok(executor.call(wasm_req).await?)
        })
    }
}
