use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use defuse_outlayer_vm_runner::{VmRuntime, host::InMemorySigner};
use lru::LruCache;

use crate::Executor;

#[must_use = "use .build()"]
pub struct ExecutorBuilder {
    compiled_cache_cap: NonZeroUsize,
}

impl Default for ExecutorBuilder {
    fn default() -> Self {
        Self {
            compiled_cache_cap: Self::DEFAULT_CACHE_CAP,
        }
    }
}

impl ExecutorBuilder {
    const DEFAULT_CACHE_CAP: NonZeroUsize = NonZeroUsize::new(1).unwrap();

    pub const fn compiled_cache_cap(mut self, cap: NonZeroUsize) -> Self {
        self.compiled_cache_cap = cap;
        self
    }

    pub fn build(self, signer: impl Into<Arc<InMemorySigner>>) -> anyhow::Result<Executor> {
        Ok(Executor {
            runtime: VmRuntime::new()?.into(),
            compiled_cache: Arc::new(Mutex::new(LruCache::new(self.compiled_cache_cap))),
            signer: signer.into(),
        })
    }
}
