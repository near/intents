mod builder;
mod cache;
mod code;
mod config;
mod resolver;

pub use self::builder::OutlayerBuilder;
pub use self::config::OutlayerConfig;
pub use self::resolver::{HttpResolver, NearResolver, Resolver, ResolverConfig, UrlResolver};
pub use self::{cache::*, code::*};

use std::{convert, sync::Arc};

use anyhow::Context as _;
use bytes::Bytes;
use defuse_outlayer_executor::{self as executor, Component, Context, Executor, HostContext, Outcome};
use defuse_outlayer_primitives::AppId;
use moka::future::Cache;

#[derive(Clone)]
pub struct Outlayer {
    resolver: Resolver,
    executor: Executor,
    runtime_cache: Cache<[u8; 32], Component>,
    default_fuel: u64,
}

impl Outlayer {
    pub fn new(
        resolver: Resolver,
        executor: Executor,
        cache: CacheConfig,
        default_fuel: u64,
    ) -> Self {
        Self {
            resolver,
            executor,
            runtime_cache: cache.build(),
            default_fuel,
        }
    }

    pub fn builder() -> OutlayerBuilder {
        OutlayerBuilder::default()
    }

    async fn resolve_app(&self, app: Code<'_>) -> Result<(AppId<'static>, Component), Error> {
        let (app_id, app_code_url, code) = match app {
            Code::Ref(code_ref) => (
                code_ref.app_id(),
                self.resolver.resolve_code_url(code_ref).await?,
                None,
            ),
            Code::Inline { code } => {
                let app_code_url = AppCodeUrl::from_code(&code);
                (app_code_url.immutable_app_id(), app_code_url, Some(code))
            }
        };

        let component = self
            .runtime_cache
            .try_get_with(app_code_url.code_hash, async move {
                let code = if let Some(code) = code {
                    code
                } else {
                    self.resolver.resolve_code(app_code_url).await?
                };

                tokio::task::spawn_blocking({
                    let compiler = self.executor.compiler();
                    move || compiler.compile(code)
                })
                .await
                .context("panicked")
                .and_then(convert::identity) // TODO: .flatten()
                .map_err(PrepareError::Compile)
            })
            .await?;

        Ok((app_id, component))
    }

    pub async fn execute(
        &self,
        app: Code<'_>,
        input: Bytes,
        fuel: Option<u64>,
    ) -> Result<Outcome, Error> {
        let (app_id, component) = self.resolve_app(app).await?;

        self.executor
            .execute(
                Context {
                    input,
                    host: HostContext { app_id },
                },
                &component,
                fuel.unwrap_or(self.default_fuel),
            )
            .await
            .map_err(Into::into)
    }
}


#[derive(derive_more::From, thiserror::Error, Debug)]
pub enum Error {
    #[from(PrepareError)]
    #[error("prepare: {0}")]
    Prepare(#[from] Arc<PrepareError>),
    #[error(transparent)]
    Execute(#[from] executor::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum PrepareError {
    #[error("resolve: {0}")]
    Resolve(#[from] resolver::Error),

    #[error("compile: {0}")]
    Compile(anyhow::Error),
}

impl From<resolver::Error> for Error {
    fn from(err: resolver::Error) -> Self {
        Self::Prepare(PrepareError::Resolve(err).into())
    }
}

#[cfg(feature = "tower")]
mod tower;
