use std::task::{Context, Poll, ready};

use defuse_outlayer_vm_runner::Component;
use futures_util::future::BoxFuture;
use tower::Service;
use tracing::Instrument as _;

use crate::{
    error::ExecutionStackError,
    executor::{self, WasmExecutionRequest},
    types::{
        AccountId, ExecutionRequest, ExecutionResponse, OffChainRequest, ProjectEnv,
        ProjectStorage, Request,
    },
};

#[derive(Clone)]
pub struct OutlayerService<E, W, Env, St, On> {
    executor: E,
    wasm: W,
    env: Env,
    storage: St,
    on_chain: On,
}

impl<E, W, Env, St, On> OutlayerService<E, W, Env, St, On> {
    pub const fn new(executor: E, wasm: W, env: Env, storage: St, on_chain: On) -> Self {
        Self {
            executor,
            wasm,
            env,
            storage,
            on_chain,
        }
    }
}

impl<E, W, Env, St, On> Service<Request> for OutlayerService<E, W, Env, St, On>
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
    On: Service<OffChainRequest, Response = ExecutionRequest> + Clone + Send + 'static,
    On::Future: Send,
    ExecutionStackError: From<On::Error>,
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

    #[tracing::instrument(level = "info", name = "outlayer.execute", skip_all, fields(
        source = tracing::field::Empty,
        request_id = tracing::field::Empty,
        project_id = tracing::field::Empty,
    ))]
    fn call(&mut self, req: Request) -> Self::Future {
        let mut executor = self.executor.clone();
        let mut wasm = self.wasm.clone();
        let mut env = self.env.clone();
        let mut storage = self.storage.clone();
        let mut on_chain = self.on_chain.clone();

        tracing::Span::current().record(
            "source",
            match &req {
                Request::OnChain(_) => "on_chain",
                Request::OffChain(_) => "off_chain",
            },
        );
        Box::pin(
            async move {
                let exec_req = match req {
                    Request::OnChain(r) => ExecutionRequest::from(r),
                    Request::OffChain(r) => on_chain.call(r).await?,
                };

                tracing::Span::current().record("request_id", &exec_req.request_id);
                tracing::Span::current().record("project_id", exec_req.project_id.0.as_str());
                tracing::debug!(wasm_url = %exec_req.wasm_url, "dispatching parallel fetches");

                let current = tracing::Span::current();
                let (component_res, env_res, storage_res) = tokio::join!(
                    wasm.call((exec_req.wasm_url.clone(), exec_req.wasm_hash))
                        .instrument(current.clone()),
                    env.call(exec_req.project_id.clone())
                        .instrument(current.clone()),
                    storage
                        .call(exec_req.project_id.clone())
                        .instrument(current),
                );

                let wasm_req = WasmExecutionRequest {
                    component: component_res?,
                    request: exec_req,
                    env: env_res?,
                    storage: storage_res?,
                    caller: None,
                };

                let response = executor.call(wasm_req).await?;
                tracing::info!(
                    instructions_used = response.metrics.instructions_used,
                    "execution complete"
                );
                Ok(response)
            }
            .instrument(tracing::Span::current()),
        )
    }
}
