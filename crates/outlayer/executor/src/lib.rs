mod builder;
mod error;

pub use self::{builder::*, error::*};

use std::sync::{Arc, Mutex};

pub use defuse_outlayer_vm_runner::host::Context as HostContext;
use defuse_outlayer_vm_runner::{
    Context as VmContext, ExecutionOutcome, VmRuntime, WasiContext,
    host::{Host, InMemorySigner},
    wasmtime::component::Component,
    wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe},
};

use bytes::Bytes;
use lru::LruCache;
use tokio::sync::OnceCell;

#[derive(Clone)]
pub struct Executor {
    runtime: Arc<VmRuntime>,
    compiled_cache: Arc<Mutex<LruCache<Bytes, OnceCell<Component>>>>,
    signer: Arc<InMemorySigner>,
}

pub struct Context {
    pub input: Bytes,
    pub host: HostContext<'static>,
}

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Outcome {
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base64::Base64"))]
    pub output: Bytes,
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base64::Base64"))]
    pub logs: Bytes,
    pub execution: ExecutionOutcome,
}

impl Outcome {
    pub fn into_result(self) -> Result<Bytes, String> {
        self.execution.into_result().map(|()| self.output)
    }
}

// TODO: maybe read from config?
const STDIN_LIMIT: usize = 4 * 1024 * 1024; // 4 MB
const STDOUT_LIMIT: usize = 4 * 1024 * 1024; // 4 MB
const STDERR_LIMIT: usize = 16 * 1024; // 16 KB

impl Executor {
    pub fn builder() -> ExecutorBuilder {
        ExecutorBuilder::default()
    }

    pub async fn execute(&self, ctx: Context, wasm: Bytes, fuel: u64) -> Result<Outcome, Error> {
        if ctx.input.len() > STDIN_LIMIT {
            // fail early, before compilation
            return Err(Error::InputTooLong);
        }

        let component = self.compile(wasm).await.map_err(Error::Compile)?;

        let stdout = MemoryOutputPipe::new(STDOUT_LIMIT);
        let stderr = MemoryOutputPipe::new(STDERR_LIMIT);
        let ctx = VmContext {
            wasi: WasiContext {
                stdin: MemoryInputPipe::new(ctx.input),
                stdout: stdout.clone(),
                stderr: stderr.clone(),
            },
            host: Host::new(ctx.host, self.signer.clone()),
            fuel,
        };

        let outcome = self
            .runtime
            .execute(ctx, &component)
            .await
            .map_err(Error::Execute)?;

        Ok(Outcome {
            output: stdout.contents(),
            logs: stderr.contents(),
            execution: outcome,
        })
    }

    async fn compile(&self, wasm: Bytes) -> anyhow::Result<Component> {
        let cached = self
            .compiled_cache
            .lock()
            .unwrap()
            .get_or_insert(wasm.clone(), OnceCell::new)
            .clone();

        cached
            .get_or_try_init(|| async { self.runtime.compile(wasm) })
            .await
            .cloned()
    }
}
