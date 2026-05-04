use std::time::Duration;

use bytes::Bytes;
use defuse_outlayer_vm_runner::wasmtime::component::Component;
use moka::sync::Cache;

#[must_use = "use .build()"]
#[derive(Debug, Clone, Default)]
pub struct CacheBuilder {
    max_capacity: Option<u64>,
    time_to_idle: Option<Duration>,
}

impl CacheBuilder {
    pub const fn max_capacity(mut self, max_capacity: u64) -> Self {
        self.max_capacity = Some(max_capacity);
        self
    }

    pub const fn time_to_idle(mut self, tti: Duration) -> Self {
        self.time_to_idle = Some(tti);
        self
    }

    pub fn build(self) -> Cache<Bytes, Component> {
        let mut builder = Cache::<Bytes, Component>::builder()
            .weigher(|wasm, _compiled| wasm.len().try_into().unwrap_or(u32::MAX));

        if let Some(max_capacity) = self.max_capacity {
            builder = builder.max_capacity(max_capacity);
        }
        if let Some(tti) = self.time_to_idle {
            builder = builder.time_to_idle(tti);
        }

        // TODO: is default hasher ok for large Bytes? or use `ahash`?
        builder.build()
    }
}
