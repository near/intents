use std::sync::Arc;

use defuse_outlayer_vm_runner::{VmRuntime, host::crypto::Signer};

use crate::Executor;

const DEFAULT_MEMORY_LIMIT: usize = 100 * 1024 * 1024; // 100 MiB
const LIMIT_4MB: usize = 4 * 1024 * 1024;
const LIMIT_16KB: usize = 16 * 1024;

#[cfg_attr(
    feature = "serde",
    derive(::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
#[derive(Debug, Clone, Copy)]
pub struct ExecutorConfig {
    pub memory_limit_bytes: usize,
    pub io_limits: IoLimits,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            memory_limit_bytes: DEFAULT_MEMORY_LIMIT,
            io_limits: IoLimits::default(),
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
#[derive(Debug, Clone, Copy)]
pub struct IoLimits {
    #[cfg_attr(feature = "serde", serde(rename = "stdin_bytes"))]
    pub stdin: usize,
    #[cfg_attr(feature = "serde", serde(rename = "stdout_bytes"))]
    pub stdout: usize,
    #[cfg_attr(feature = "serde", serde(rename = "stderr_bytes"))]
    pub stderr: usize,
}

impl Default for IoLimits {
    fn default() -> Self {
        Self {
            stdin: LIMIT_4MB,
            stdout: LIMIT_4MB,
            stderr: LIMIT_16KB,
        }
    }
}

impl ExecutorConfig {
    pub fn build(self, signer: Arc<dyn Signer>) -> anyhow::Result<Executor> {
        Ok(Executor::new(
            signer,
            Arc::new(VmRuntime::new(self.memory_limit_bytes)?),
            self.io_limits,
        ))
    }
}
