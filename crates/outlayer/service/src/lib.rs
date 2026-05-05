mod code;
mod resolver;

pub use self::code::*;

use std::sync::Arc;

use bytes::Bytes;
use defuse_outlayer_executor::{self as executor, Component, Context, Executor, Outcome};

use crate::resolver::Resolver;
use moka::future::Cache;

#[derive(Clone)]
pub struct Outlayer {
    resolver: Resolver,
    executor: Executor,
    runtime_cache: Cache<AppCodeUrl, Component>,
}

impl Outlayer {

    async fn compile(&self, url: AppCodeUrl) -> Result<Component, Error> {
        let resolver = self.resolver.clone();
        let compiler = self.executor.compiler();
        self.runtime_cache
            .try_get_with(url.clone(), async move {
                let wasm = resolver
                    .resolve_code(CodeRef::Url(url))
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                tokio::task::spawn_blocking(move || compiler.compile(wasm))
                    .await
                    .map_err(|e| anyhow::anyhow!("compile panicked: {e}"))?
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
            .await
            .map_err(Error::Compile)
    }

    pub async fn execute(&self, app: Code<'_>, input: Bytes, fuel: u64) -> Result<Outcome, Error> {
        let app_code_url = self.resolver.resolve_app_id(app.app_id()).await?;
        let component = self.compile(app_code_url).await?;
        self.executor
            .execute(
                Context {
                    input,
                    host: todo!("HostContext"),
                },
                &component,
                fuel,
            )
            .await
            .map_err(Error::Execute)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("resolve: {0}")]
    Resolve(#[from] resolver::Error),
    #[error("compile: {0}")]
    Compile(Arc<anyhow::Error>),
    #[error(transparent)]
    Execute(#[from] executor::Error),
}
