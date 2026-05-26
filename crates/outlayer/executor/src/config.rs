use std::sync::Arc;

use defuse_outlayer_vm_runner::{VmRuntime, host::crypto::Signer};

use crate::Executor;

const LIMIT_4MB: usize = 4 * 1024 * 1024;
const LIMIT_16KB: usize = 16 * 1024;

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
#[derive(Debug, Clone, Copy)]
pub struct ExecutorConfig {
    pub memory_limit: usize,
    pub limits: StreamLimits,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            memory_limit: VmRuntime::DEFAULT_MEMORY_LIMIT,
            limits: StreamLimits::default(),
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
#[derive(Debug, Clone, Copy)]
pub struct StreamLimits {
    pub stdin: usize,
    pub stdout: usize,
    pub stderr: usize,
}

impl Default for StreamLimits {
    fn default() -> Self {
        Self {
            stdin: LIMIT_4MB,
            stdout: LIMIT_4MB,
            stderr: LIMIT_16KB,
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
