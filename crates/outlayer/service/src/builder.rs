use defuse_outlayer_executor::Executor;

use crate::{CacheBuilder, Outlayer, Resolver};

#[must_use = "use .build()"]
#[derive(Default)]
pub struct OutlayerBuilder {
    cache: CacheBuilder,
}

impl OutlayerBuilder {
    pub const fn with_cache(mut self, cache: CacheBuilder) -> Self {
        self.cache = cache;
        self
    }

    pub fn build(self, resolver: Resolver, executor: Executor) -> Outlayer {
        Outlayer {
            resolver,
            executor,
            runtime_cache: self.cache.build(),
        }
    }
}
