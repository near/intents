use std::sync::Arc;

use defuse_outlayer_vm_runner::{VmRuntime, host::InMemorySigner};

use crate::Executor;

#[must_use = "use .build()"]
#[derive(Debug, Clone, Default)]
pub struct ExecutorBuilder;

impl ExecutorBuilder {
    pub fn build(self, signer: impl Into<Arc<InMemorySigner>>) -> anyhow::Result<Executor> {
        Ok(Executor {
            runtime: VmRuntime::new()?.into(),
            signer: signer.into(),
        })
    }
}
