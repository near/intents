use std::sync::Arc;

use defuse_outlayer_vm_runner::{VmRuntime, host::crypto::Signer};

use crate::Executor;

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
#[derive(Debug, Clone, Copy)]
pub struct ExecutorConfig {
    pub memory_limit: usize,
    pub limits: ExecutorLimits,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            memory_limit: VmRuntime::DEFAULT_MEMORY_LIMIT,
            limits: ExecutorLimits::default(),
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
#[derive(Debug, Clone, Copy)]
pub struct ExecutorLimits {
    pub stdin: usize,
    pub stdout: usize,
    pub stderr: usize,
}

impl Default for ExecutorLimits {
    fn default() -> Self {
        Self {
            stdin: 4 * 1024 * 1024,
            stdout: 4 * 1024 * 1024,
            stderr: 16 * 1024,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExecutorBuilder {
    config: ExecutorConfig,
}

impl ExecutorBuilder {
    #[must_use]
    pub const fn with_config(mut self, config: ExecutorConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self, signer: Arc<dyn Signer>) -> anyhow::Result<Executor> {
        Ok(Executor::new(
            signer,
            Arc::new(VmRuntime::new(self.config.memory_limit)?),
            self.config.limits,
        ))
    }
}
