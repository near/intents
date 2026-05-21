use std::sync::Arc;

use defuse_outlayer_executor::{Executor, InMemorySigner};
use near_kit::Near;

use crate::{HttpResolver, NearResolver, Outlayer, OutlayerConfig, Resolver, UrlResolver};

#[derive(Default)]
pub struct OutlayerBuilder {
    config: OutlayerConfig,
}

impl OutlayerBuilder {
    pub fn with_config(mut self, config: OutlayerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self, signer: impl Into<Arc<InMemorySigner>>) -> anyhow::Result<Outlayer> {
        let executor = Executor::new(signer, self.config.executor)?;
        let near =
            Near::custom(self.config.resolver.near_rpc_url, self.config.resolver.near_chain_id)
                .build();
        let resolver = Resolver::new(
            NearResolver::new(near),
            UrlResolver::new(HttpResolver::new(self.config.resolver.http_max_len)),
        );
        Ok(Outlayer::new(
            resolver,
            executor,
            self.config.cache,
            self.config.default_fuel,
        ))
    }

}
