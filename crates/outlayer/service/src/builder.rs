use std::sync::Arc;

use defuse_outlayer_executor::{ExecutorBuilder, Signer};

use crate::{CacheBuilder, Outlayer, OutlayerConfig, ResolverBuilder};

#[must_use = "call .build() to construct the Outlayer"]
#[derive(Default)]
pub struct OutlayerBuilder {
    config: OutlayerConfig,
}

impl OutlayerBuilder {
    pub fn with_config(mut self, config: OutlayerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self, signer: impl Into<Arc<dyn Signer>>) -> anyhow::Result<Outlayer> {
        let executor = ExecutorBuilder::default()
            .with_config(self.config.executor)
            .build(signer.into())?;
        let resolver = ResolverBuilder::default()
            .with_config(self.config.resolver)
            .build();
        let cache = CacheBuilder::default()
            .with_config(self.config.cache)
            .build();
        Ok(Outlayer::new(
            resolver,
            executor,
            cache,
            self.config.default_fuel,
        ))
    }
}
