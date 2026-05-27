use std::time::Duration;

use defuse_outlayer_executor::Component;
use moka::future::Cache;

#[cfg_attr(
    feature = "serde",
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
pub struct CacheConfig {
    pub max_capacity_bytes: u64,
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "Option<::serde_with::DurationSeconds<u64>>")
    )]
    pub time_to_idle: Option<Duration>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity_bytes: 100 * 1024 * 1024,
            time_to_idle: None,
        }
    }
}

#[derive(Default)]
pub struct CacheBuilder(CacheConfig);

impl CacheBuilder {
    #[must_use]
    pub const fn with_config(mut self, config: CacheConfig) -> Self {
        self.0 = config;
        self
    }

    pub fn build(self) -> Cache<[u8; 32], Component> {
        let mut builder = Cache::<[u8; 32], Component>::builder()
            .max_capacity(self.0.max_capacity_bytes)
            .weigher(|_hash, comp: &Component| {
                // Approximates the in-memory size of a compiled component using its
                // mmap'd image range. Per-Component heap metadata (type tables, etc.)
                // lives outside this range and is not counted.
                let r = comp.image_range();
                u32::try_from((r.start.addr()..r.end.addr()).len()).unwrap_or(u32::MAX)
            });
        if let Some(tti) = self.0.time_to_idle {
            builder = builder.time_to_idle(tti);
        }
        builder.build()
    }
}
