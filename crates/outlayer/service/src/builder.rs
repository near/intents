use std::sync::Arc;

use defuse_outlayer_executor::{Executor, InMemorySigner};

use crate::{Outlayer, OutlayerConfig, Resolver};

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
        let signer: Arc<InMemorySigner> = signer.into();
        let executor = Executor::new(signer, self.config.executor)?;
        let resolver = Resolver::new(self.config.resolver);
        Ok(Outlayer::new(
            resolver,
            executor,
            self.config.cache,
            self.config.default_fuel,
        ))
    }

    #[cfg(feature = "tower")]
    pub fn build_service(
        self,
        signer: impl Into<Arc<InMemorySigner>>,
    ) -> anyhow::Result<
        //TODO: simplify and maybe get rid of BoxClone ?
        tower::util::BoxCloneService<
            (crate::Code<'static>, bytes::Bytes),
            defuse_outlayer_executor::Outcome,
            Box<dyn std::error::Error + Send + Sync>,
        >,
    > {
        let pool_config = self.config.tower;
        let outlayer = self.build(signer)?;
        let svc = tower::service_fn(
            move |(app, input): (crate::Code<'static>, bytes::Bytes)| {
                let outlayer = outlayer.clone();
                async move {
                    outlayer
                        .execute(app, input, None)
                        .await
                        .map_err(|e: crate::Error| {
                            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                        })
                }
            },
        );
        Ok(crate::tower::wrap_with_pool(svc, pool_config))
    }
}
