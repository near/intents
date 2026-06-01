use std::sync::Arc;

use defuse_outlayer_executor::{ExecutorConfig, Signer};

use crate::{CacheConfig, Outlayer, resolver::ResolverConfig};
const DEFAULT_FUEL: u64 = 1_000_000_000;

#[cfg_attr(
    feature = "serde",
    derive(::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
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
            // TODO: determine a reasonable fuel ceiling based on benchmarked workloads
            default_fuel: DEFAULT_FUEL,
        }
    }
}

impl OutlayerConfig {
    pub fn build(self, signer: impl Into<Arc<dyn Signer>>) -> anyhow::Result<Outlayer> {
        let executor = self.executor.build(signer.into())?;
        let resolver = self.resolver.build();
        let cache = self.cache.build();
        Ok(Outlayer::new(resolver, executor, cache, self.default_fuel))
    }
}
