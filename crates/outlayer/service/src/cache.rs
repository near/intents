use std::time::Duration;

use defuse_outlayer_executor::Component;
use moka::future::Cache;

#[must_use = "use .build()"]
#[derive(Debug, Clone)]
pub struct CacheBuilder {
    max_capacity: u64,
    time_to_idle: Option<Duration>,
}

impl Default for CacheBuilder {
    fn default() -> Self {
        Self {
            max_capacity: 0,
            time_to_idle: None,
        }
    }
}

impl CacheBuilder {
    pub const fn max_capacity(mut self, max_capacity: u64) -> Self {
        self.max_capacity = max_capacity;
        self
    }

    pub const fn time_to_idle(mut self, tti: Duration) -> Self {
        self.time_to_idle = Some(tti);
        self
    }

    pub fn build(self) -> Cache<[u8; 32], Component> {
        let mut builder = Cache::<[u8; 32], Component>::builder().max_capacity(self.max_capacity);
        if let Some(tti) = self.time_to_idle {
            builder = builder.time_to_idle(tti);
        }
        builder.build()
    }
}
