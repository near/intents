use std::task::{ready, Context, Poll};

use bytes::Bytes;
use futures_util::future::BoxFuture;
use sha2::Digest;
use tower::Service;

use crate::{
    error::ExecutionStackError,
    executor,
    resolver::ResolveError,
    types::{
        AccountId, CompiledWasmRuntime, ExecutionResponse, ProjectEnv,
        ProjectStorage, Request, ResourceLimits, WasmExecutionRequest,
    },
};

pub struct TeeWorkerService<E, R, Env, St> {
    executor: E,
    resolver: R,
    env:      Env,
    storage:  St,
}

impl<E, R, Env, St> TeeWorkerService<E, R, Env, St> {
    pub const fn new(executor: E, resolver: R, env: Env, storage: St) -> Self {
        Self { executor, resolver, env, storage }
    }
}

impl<E, R, Env, St> Service<Request> for TeeWorkerService<E, R, Env, St>
where
    E: Service<WasmExecutionRequest, Response = ExecutionResponse, Error = executor::WasmExecutorError>
        + Send
        + Clone
        + 'static,
    E::Future: Send,
    R: Service<String, Response = Bytes, Error = ExecutionStackError>
        + Send
        + Clone
        + 'static,
    R::Future: Send,
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
        ready!(self.resolver.poll_ready(cx))?;
        ready!(self.env.poll_ready(cx))?;
        ready!(self.storage.poll_ready(cx))?;
        ready!(self.executor.poll_ready(cx)).map_err(ExecutionStackError::from)?;
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut executor = self.executor.clone();
        let mut resolver = self.resolver.clone();
        let mut env     = self.env.clone();
        let mut storage = self.storage.clone();

        Box::pin(async move {
            let Request::OnChain(onchain) = req else {
                todo!("offchain not yet implemented")
            };

            let (bytes_res, env_res, storage_res) = tokio::join!(
                resolver.call(onchain.wasm_url.clone()),
                env.call(onchain.project_id.clone()),
                storage.call(onchain.project_id.clone()),
            );

            let bytes = bytes_res?;
            verify_hash(&bytes, &onchain.wasm_hash)?;

            let wasm_req = WasmExecutionRequest {
                request: onchain,
                runtime: CompiledWasmRuntime, // TODO: compile from bytes via wasmtime
                env: env_res?,
                storage: storage_res?,
                caller: None, // TODO: verify via NEAR RPC
                limits: ResourceLimits,
            };

            let response = executor.call(wasm_req).await?;
            Ok(response)
        })
    }
}

fn verify_hash(bytes: &Bytes, expected: &[u8; 32]) -> Result<(), ExecutionStackError> {
    let actual: [u8; 32] = sha2::Sha256::digest(bytes).into();
    if actual != *expected {
        return Err(ExecutionStackError::Fetch(ResolveError::HashMismatch {
            expected: hex_encode(expected),
            actual:   hex_encode(&actual),
        }));
    }
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        write!(s, "{b:02x}").unwrap();
        s
    })
}
