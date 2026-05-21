use defuse_outlayer_executor::Config as ExecutorConfig;

use crate::{CacheConfig, resolver::ResolverConfig};

#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize)
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OutlayerConfig {
    pub executor: ExecutorConfig,
    pub resolver: ResolverConfig,
    pub cache: CacheConfig,
    pub default_fuel: u64,
}

impl Default for OutlayerConfig {
    fn default() -> Self {
        Self {
            executor: ExecutorConfig::default(),
            resolver: ResolverConfig::default(),
            cache: CacheConfig::default(),
            default_fuel: u64::MAX,
        }
    }
}
