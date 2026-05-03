mod error;
mod request;
mod response;

pub use self::{error::*, request::*, response::*};

use std::{
    num::NonZeroUsize,
    sync::Arc,
    task::{self, Poll},
};

use bytes::Bytes;
use defuse_outlayer_vm_runner::{
    Context as VmContext, VmRuntime, WasiContext,
    host::{Host, InMemorySigner},
    wasmtime::component::Component,
    wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe},
};
use futures::{FutureExt, future::BoxFuture};
use lru::LruCache;
use tower::Service;
use tracing::Instrument;

pub struct Executor {
    runtime: Arc<VmRuntime>,
    compiled: LruCache<Bytes, Component>,
    signer: Arc<InMemorySigner>,
}

pub struct ExecutorBuilder {
    cache_cap: NonZeroUsize,
}

impl Default for ExecutorBuilder {
    fn default() -> Self {
        Self {
            cache_cap: Self::DEFAULT_CACHE_CAP,
        }
    }
}

impl ExecutorBuilder {
    const DEFAULT_CACHE_CAP: NonZeroUsize = NonZeroUsize::new(1).unwrap();

    pub fn cache_cap(mut self, cap: NonZeroUsize) -> Self {
        self.cache_cap = cap;
        self
    }

    pub fn build(self, signer: impl Into<Arc<InMemorySigner>>) -> anyhow::Result<Executor> {
        Ok(Executor {
            runtime: VmRuntime::new()?.into(),
            compiled: LruCache::new(self.cache_cap),
            signer: signer.into(),
        })
    }
}

// TODO: maybe read from config?
const STDIN_LIMIT: usize = 4 * 1024 * 1024; // 4 MB
const STDOUT_LIMIT: usize = 4 * 1024 * 1024; // 4 MB
const STDERR_LIMIT: usize = 16 * 1024; // 16 KB

impl Executor {
    fn execute(
        &mut self,
        req: Request,
    ) -> Result<BoxFuture<'static, Result<Response, Error>>, Error> {
        if req.ctx.input.len() > STDIN_LIMIT {
            return Err(Error::InputTooLong);
        }

        let component = self
            .compiled
            .try_get_or_insert_with_key(req.wasm, |binary| {
                self.runtime.compile(binary).map_err(Error::Compile)
            })
            .cloned()?;

        let ctx = VmContext {
            wasi: WasiContext {
                stdin: MemoryInputPipe::new(req.ctx.input),
                stdout: MemoryOutputPipe::new(STDOUT_LIMIT),
                stderr: MemoryOutputPipe::new(STDERR_LIMIT),
            },
            host: Host::new(req.ctx.host, self.signer.clone()),
            fuel: req.fuel,
        };

        let runtime = self.runtime.clone();
        Ok(async move {
            let stdout = ctx.wasi.stdout.clone();
            let stderr = ctx.wasi.stderr.clone();

            let outcome = runtime
                .execute(ctx, &component)
                .await
                .map_err(Error::Execute)?;

            Ok(Response {
                output: stdout.contents(),
                logs: stderr.contents(),
                outcome,
            })
        }
        .in_current_span()
        .boxed())
    }
}

impl Service<Request> for Executor {
    type Response = Response;

    type Error = Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.execute(req)
            .unwrap_or_else(|err| futures::future::ready(Err(err)).boxed())
    }
}
