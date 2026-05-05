use std::sync::Arc;

use bytes::Bytes;
use defuse_outlayer_vm_runner::{VmRuntime, wasmtime::component::Component};

#[derive(Clone)]
pub struct Compiler(Arc<VmRuntime>);

impl Compiler {
    pub(super) const fn new(runtime: Arc<VmRuntime>) -> Self {
        Self(runtime)
    }

    pub fn compile(&self, wasm: Bytes) -> anyhow::Result<Component> {
        self.0.compile(wasm)
    }
}
