use std::time::Duration;

use defuse_outlayer_executor::Component;
use moka::future::Cache;

#[cfg(feature = "serde")]
struct Clamp<const MIN: i64, const MAX: i64>;

#[cfg(feature = "serde")]
impl<'de, const MIN: i64, const MAX: i64> ::serde_with::DeserializeAs<'de, u64>
    for Clamp<MIN, MAX>
{
    fn deserialize_as<D: ::serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        use ::serde::Deserialize as _;
        let v = i64::deserialize(d)?.clamp(MIN, MAX);
        u64::try_from(v).map_err(::serde::de::Error::custom)
    }
}

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
pub struct CacheConfig {
    #[cfg_attr(
        feature = "serde",
        serde_as(deserialize_as = "Clamp<{ 1024 * 1024 }, { 10 * 1024 * 1024 * 1024 }>")
    )]
    pub max_capacity: u64,

    pub tti_secs: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 100 * 1024 * 1024,
            tti_secs: None,
        }
    }
}

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
