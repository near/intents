mod builder;
mod cache;
mod code;
mod config;
#[cfg(feature = "proto")]
mod proto_impls;
#[cfg(feature = "tonic")]
mod tonic_impl;
#[cfg(feature = "tower")]
mod tower_impl;
mod resolver;

pub use self::builder::OutlayerBuilder;
pub use self::config::OutlayerConfig;
pub use self::resolver::{
    HttpResolver, NearResolver, Resolver, ResolverBuilder, ResolverConfig, UrlResolver,
};
pub use self::{cache::*, code::*};
#[cfg(feature = "tonic")]
pub use tonic_impl::OutlayerGrpc;
#[cfg(feature = "tower")]
pub use tower_impl::ExecuteRequest;

use std::{convert, sync::Arc};

use anyhow::Context as _;
use bytes::Bytes;
use defuse_outlayer_executor::{
    self as executor, Component, Context, Executor, HostContext, Outcome,
};
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
    pub const fn new(
        resolver: Resolver,
        executor: Executor,
        runtime_cache: Cache<[u8; 32], Component>,
        default_fuel: u64,
    ) -> Self {
        Self {
            resolver,
            executor,
            runtime_cache,
            default_fuel,
        }
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

        let executor = self.executor.clone();
        let default_fuel = self.default_fuel;
        // WASM execution is CPU-bound and wasmtime doesn't yield between instructions
        // without epoch interruption, which would block the tokio scheduler.
        // Run on the blocking thread pool to keep the async runtime responsive.
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(executor.execute(
                Context {
                    input,
                    host: HostContext { app_id },
                },
                &component,
                fuel.unwrap_or(default_fuel),
            ))
        })
        .await
        .map_err(Error::ExecutePanicked)?
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
    #[error("execute panicked: {0}")]
    ExecutePanicked(#[from] tokio::task::JoinError),
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
