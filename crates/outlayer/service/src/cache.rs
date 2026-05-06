use std::time::Duration;

use defuse_outlayer_executor::Component;
use moka::future::Cache;

const DEFAULT_MAX_CAPACITY: u64 = 100 * 1024 * 1024; // 100MB

#[must_use = "use .build()"]
#[derive(Debug, Clone)]
pub struct CacheBuilder {
    max_capacity: u64,
    time_to_idle: Option<Duration>,
}

impl Default for CacheBuilder {
    fn default() -> Self {
        Self {
            max_capacity: DEFAULT_MAX_CAPACITY,
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
        let mut builder = Cache::<[u8; 32], Component>::builder()
            .max_capacity(self.max_capacity)
            .weigher(|_hash, comp: &Component| {
                // Approximates the in-memory size of a compiled component using its
                // mmap'd image range. Per-Component heap metadata (type tables, etc.)
                // lives outside this range and is not counted.
                let r = comp.image_range();
                u32::try_from((r.start.addr()..r.end.addr()).len()).unwrap_or(u32::MAX)
            });
        if let Some(tti) = self.time_to_idle {
            builder = builder.time_to_idle(tti);
        }
        builder.build()
    }
}
