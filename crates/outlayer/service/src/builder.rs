use std::sync::Arc;

use defuse_outlayer_executor::{Executor, Signer};

use crate::{Outlayer, OutlayerConfig, Resolver};

#[derive(Default)]
pub struct OutlayerBuilder {
    config: OutlayerConfig,
}

impl OutlayerBuilder {
    #[must_use]
    pub fn with_config(mut self, config: OutlayerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self, signer: impl Into<Arc<dyn Signer>>) -> anyhow::Result<Outlayer> {
        let executor = Executor::builder()
            .with_config(self.config.executor)
            .build(signer.into())?;
        let resolver = Resolver::build(self.config.resolver);
        let cache = self.config.cache.build();
        Ok(Outlayer::new(
            resolver,
            executor,
            cache,
            self.config.default_fuel,
        ))
    }
}
