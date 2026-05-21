use std::time::Duration;

use defuse_outlayer_executor::Component;
use moka::future::Cache;

const MB: i64 = 1024 * 1024;

#[cfg(feature = "serde")]
use defuse_outlayer_utils::Clamp;

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CacheConfig {
    #[cfg_attr(
        feature = "serde",
        serde_as(deserialize_as = "Clamp<{ 1 * MB }, { 10 * 1024 * MB }, u64>")
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

impl CacheConfig {
    pub fn build(self) -> Cache<[u8; 32], Component> {
        let mut builder = Cache::<[u8; 32], Component>::builder()
            .max_capacity(self.max_capacity)
            .weigher(|_hash, comp: &Component| {
                let r = comp.image_range();
                u32::try_from((r.start.addr()..r.end.addr()).len()).unwrap_or(u32::MAX)
            });
        if let Some(tti) = self.tti_secs {
            builder = builder.time_to_idle(Duration::from_secs(tti));
        }
        builder.build()
    }
}
