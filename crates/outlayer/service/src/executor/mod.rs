mod request;
mod response;

pub use self::{request::*, response::*};

use std::{
    sync::Arc,
    task::{self, Poll},
};

use defuse_outlayer_vm_runner::{
    Context as VmContext, VmRuntime, WasiContext,
    host::{InMemorySigner, State},
    wasmtime::Error,
    wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe},
};
use futures::{FutureExt, future::BoxFuture};
use tower::Service;
use tracing::Instrument;

pub struct Executor {
    runtime: Arc<VmRuntime>,
    signer: Arc<InMemorySigner>,
}

const STDOUT_LIMIT: usize = 4 * 1024 * 1024; // 4 MB
const STDERR_LIMIT: usize = 16 * 1024; // 16 KB

impl Service<Request> for Executor {
    type Response = Response;

    type Error = Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let runtime = self.runtime.clone();
        let signer = self.signer.clone();

        async move {
            let stdout = MemoryOutputPipe::new(STDOUT_LIMIT);
            let stderr = MemoryOutputPipe::new(STDERR_LIMIT);

            let outcome = runtime
                .execute(
                    VmContext {
                        wasi: WasiContext {
                            stdin: MemoryInputPipe::new(req.ctx.input),
                            stdout: stdout.clone(),
                            stderr: stderr.clone(),
                        },
                        host_state: State::new(req.ctx.host, signer),
                        fuel: req.fuel,
                    },
                    &req.component,
                )
                .await?;

            Ok(Response {
                output: stdout.contents(),
                logs: stderr.contents(),
                outcome,
            })
        }
        .in_current_span()
        .boxed()
    }
}
