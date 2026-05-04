use std::sync::Arc;

use defuse_outlayer_vm_runner::{VmRuntime, host::InMemorySigner};

use crate::{CacheBuilder, Executor};

#[must_use = "use .build()"]
#[derive(Debug, Clone, Default)]
pub struct ExecutorBuilder {
    compile_cache: CacheBuilder,
}

impl ExecutorBuilder {
    pub fn build(self, signer: impl Into<Arc<InMemorySigner>>) -> anyhow::Result<Executor> {
        Ok(Executor {
            runtime: VmRuntime::new()?.into(),
            compiled_cache: self.compile_cache.build(),
            signer: signer.into(),
        })
    }
}
