use std::sync::Arc;

use defuse_outlayer_vm_runner::{VmRuntime, wasmtime::component::Component};

#[derive(Clone)]
pub struct Compiler(Arc<VmRuntime>);

impl Compiler {
    pub(super) const fn new(runtime: Arc<VmRuntime>) -> Self {
        Self(runtime)
    }

    #[tracing::instrument(name = "compile", level = "debug", skip_all)]
    pub fn compile(&self, wasm: impl AsRef<[u8]>) -> anyhow::Result<Component> {
        self.0.compile(wasm)
    }
}
