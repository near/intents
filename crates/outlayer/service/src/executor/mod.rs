mod request;
mod response;

pub use self::{request::*, response::*};

use std::{
    num::NonZeroUsize,
    sync::Arc,
    task::{self, Poll},
};

use anyhow::Context as _;
use bytes::Bytes;
use defuse_outlayer_vm_runner::{
    Context as VmContext, VmRuntime, WasiContext,
    host::{Host, InMemorySigner},
    wasmtime::{Error, component::Component},
    wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe},
};
use futures::{FutureExt, future::BoxFuture};
use lru::LruCache;
use tower::Service;
use tracing::Instrument;

pub struct Executor {
    runtime: Arc<VmRuntime>,
    cache: LruCache<Bytes, Component>,
    // TODO: add LRU cache for components
    signer: Arc<InMemorySigner>,
}

pub struct ExecutorBuilder {
    component_cache_cap: NonZeroUsize,
}

impl Default for ExecutorBuilder {
    fn default() -> Self {
        Self {
            component_cache_cap: NonZeroUsize::new(1).unwrap_or_else(|| unreachable!()),
        }
    }
}

impl ExecutorBuilder {
    pub fn cache_components(mut self, cap: NonZeroUsize) -> Self {
        self.component_cache_cap = cap;
        self
    }

    pub fn build(self, signer: impl Into<Arc<InMemorySigner>>) -> anyhow::Result<Executor> {
        Ok(Executor {
            runtime: VmRuntime::new()?.into(),
            cache: LruCache::new(self.component_cache_cap),
            signer: signer.into(),
        })
    }
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
        let component = match self
            .cache
            .try_get_or_insert_with_key(req.wasm, |binary| {
                self.runtime.compile(binary).context("compile")
            })
            .cloned()
        {
            Ok(component) => component,
            Err(err) => return futures::future::ready(Err(err)).boxed(),
        };

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
        async move {
            let stdout = ctx.wasi.stdout.clone();
            let stderr = ctx.wasi.stderr.clone();

            let outcome = runtime.execute(ctx, &component).await?;

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
