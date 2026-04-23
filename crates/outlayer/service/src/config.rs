use std::time::Duration;

use crate::executor::WasmExecutorConfig;
use crate::utils::cache::CacheConfig;

#[derive(Debug, Clone)]
pub struct FetchConfig {
    pub timeout: Duration,
    pub retry_attempts: usize,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            retry_attempts: 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub total_timeout: Duration,
    pub cache: CacheConfig,
    pub fetch: FetchConfig,
    pub executor: WasmExecutorConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            total_timeout: Duration::from_secs(30),
            cache: CacheConfig::default(),
            fetch: FetchConfig::default(),
            executor: WasmExecutorConfig::default(),
        }
    }
}
