use std::sync::Arc;

use defuse_outlayer_vm_runner::{VmRuntime, wasmtime::component::Component};

#[derive(Clone)]
pub struct Compiler(Arc<VmRuntime>);

impl Compiler {
    pub(super) const fn new(runtime: Arc<VmRuntime>) -> Self {
        Self(runtime)
    }

    pub fn compile(&self, wasm: impl AsRef<[u8]>) -> Result<Component, CompileError> {
        Ok(self.0.compile(wasm)?)
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct CompileError(#[from] pub anyhow::Error);
