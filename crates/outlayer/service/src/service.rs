use std::task::{Context, Poll, ready};

use futures_util::future::BoxFuture;
use tower::Service;
use wasmtime::component::Component;

use crate::{
    error::ExecutionStackError,
    executor::{self, WasmExecutionRequest},
    types::{
        AccountId, ExecutionRequest, ExecutionResponse, OffChainRequest, ProjectEnv, ProjectStorage,
        Request,
    },
};

#[derive(Clone)]
pub struct OutlayerService<E, W, Env, St, F> {
    executor: E,
    wasm: W,
    env: Env,
    storage: St,
    fetch: F,
}

impl<E, W, Env, St, F> OutlayerService<E, W, Env, St, F> {
    pub const fn new(executor: E, wasm: W, env: Env, storage: St, fetch: F) -> Self {
        Self {
            executor,
            wasm,
            env,
            storage,
            fetch,
        }
    }
}

impl<E, W, Env, St, F> Service<Request> for OutlayerService<E, W, Env, St, F>
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
    F: Service<OffChainRequest, Response = ExecutionRequest> + Clone + Send + 'static,
    F::Future: Send,
    ExecutionStackError: From<F::Error>,
{
    type Response = ExecutionResponse;
    type Error = ExecutionStackError;
    type Future = BoxFuture<'static, Result<ExecutionResponse, ExecutionStackError>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ExecutionStackError>> {
        ready!(self.wasm.poll_ready(cx))?;
        ready!(self.env.poll_ready(cx))?;
        ready!(self.storage.poll_ready(cx))?;
        ready!(self.executor.poll_ready(cx)).map_err(ExecutionStackError::Executor)?;
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut executor = self.executor.clone();
        let mut wasm = self.wasm.clone();
        let mut env = self.env.clone();
        let mut storage = self.storage.clone();
        let mut fetch = self.fetch.clone();

        Box::pin(async move {
            let exec_req = match req {
                Request::OnChain(r) => ExecutionRequest::from(r),
                Request::OffChain(r) => fetch.call(r).await.map_err(Into::into)?,
            };

            let (component_res, env_res, storage_res) = tokio::join!(
                wasm.call((exec_req.wasm_url.clone(), exec_req.wasm_hash)),
                env.call(exec_req.project_id.clone()),
                storage.call(exec_req.project_id.clone()),
            );

            let wasm_req = WasmExecutionRequest {
                component: component_res?,
                request: exec_req,
                env: env_res?,
                storage: storage_res?,
                caller: None,
            };

            Ok(executor.call(wasm_req).await?)
        })
    }
}
