use std::{error::Error, num::NonZeroUsize};

use serde::{Deserialize, Serialize};
use tower::Service;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct WorkerPoolConfig {
    pub buffer: NonZeroUsize,
    pub concurrency: usize,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            buffer: NonZeroUsize::new(1).unwrap(),
            concurrency: 3,
        }
    }
}

pub struct WorkerPoolBuilder {
    config: WorkerPoolConfig,
}

impl WorkerPoolBuilder {
    pub fn new(config: WorkerPoolConfig) -> Self {
        Self { config }
    }

    pub fn build<S, Req>(
        self,
        svc: S,
    ) -> tower::util::BoxCloneService<Req, S::Response, Box<dyn Error + Send + Sync>>
    where
        S: Service<Req> + Clone + Send + 'static,
        S::Future: Send,
        S::Error: Into<Box<dyn Error + Send + Sync>> + Send + Sync,
        Req: Send + 'static,
    {
        tower::util::BoxCloneService::new(
            tower::ServiceBuilder::new()
                .buffer(self.config.buffer.get())
                .concurrency_limit(self.config.concurrency)
                .service(svc),
        )
    }
}
