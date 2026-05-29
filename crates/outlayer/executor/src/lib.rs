mod compiler;
mod config;
mod error;

pub use self::{compiler::*, config::*, error::*};

use std::sync::Arc;

pub use defuse_outlayer_vm_runner::host::{Context as HostContext, crypto::Signer};
pub use defuse_outlayer_vm_runner::wasmtime::component::Component;

use defuse_outlayer_vm_runner::{
    Context as VmContext, ExecutionOutcome, VmRuntime, WasiContext,
    host::Host,
    wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe},
};

use bytes::Bytes;

#[derive(Clone)]
pub struct Executor {
    runtime: Arc<VmRuntime>,
    signer: Arc<dyn Signer>,
    limits: IoLimits,
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

impl Executor {
    pub fn new(signer: Arc<dyn Signer>, runtime: Arc<VmRuntime>, limits: IoLimits) -> Self {
        Self {
            runtime,
            signer,
            limits,
        }
    }

    pub fn compiler(&self) -> Compiler {
        Compiler::new(self.runtime.clone())
    }

    pub async fn execute(
        &self,
        ctx: Context,
        component: &Component,
        fuel: u64,
    ) -> Result<Outcome, Error> {
        if ctx.input.len() > self.limits.stdin {
            return Err(Error::InputTooLong);
        }

        let stdout = MemoryOutputPipe::new(self.limits.stdout);
        let stderr = MemoryOutputPipe::new(self.limits.stderr);
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
            .execute(ctx, component)
            .await
            .map_err(Error::Execute)?;

        Ok(Outcome {
            output: stdout.contents(),
            logs: stderr.contents(),
            execution: outcome,
        })
    }
}
