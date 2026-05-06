mod cache;
mod code;
mod resolver;

pub use self::resolver::Resolver;
pub use self::{cache::*, code::*};

use std::sync::Arc;

use bytes::Bytes;
use defuse_outlayer_executor::{
    self as executor, Component, Context, Executor, HostContext, Outcome,
};
use moka::future::Cache;

#[derive(Clone)]
pub struct Outlayer {
    resolver: Resolver,
    executor: Executor,
    runtime_cache: Option<Cache<Bytes, Component>>,
}

impl Outlayer {
    pub const fn new(
        resolver: Resolver,
        executor: Executor,
        cache: Option<Cache<Bytes, Component>>,
    ) -> Self {
        Self {
            resolver,
            executor,
            runtime_cache: cache,
        }
    }

    async fn resolve(&self, app: Code<'_>) -> Result<(HostContext<'static>, Bytes), Error> {
        match app {
            Code::Ref(code_ref) => Ok((
                HostContext {
                    app_id: code_ref.app_id(),
                },
                self.resolver.resolve_code(code_ref).await?,
            )),
            Code::Inline { code } => {
                let app_code_url = AppCodeUrl::from_code(&code);
                Ok((
                    HostContext {
                        app_id: app_code_url.immutable_app_id(),
                    },
                    code,
                ))
            }
        }
    }

    async fn compile(&self, code: Bytes) -> Result<Component, Error> {
        let do_compile = |code| async {
            let compiler = self.executor.compiler();
            tokio::task::spawn_blocking(move || compiler.compile(code))
                .await
                .map_err(|e| anyhow::anyhow!("compile panicked: {e}"))?
        };

        if let Some(cache) = &self.runtime_cache {
            cache.try_get_with(code.clone(), do_compile(code)).await
        } else {
            do_compile(code).await.map_err(Arc::new)
        }
        .map_err(Error::Compile)
    }

    pub async fn execute(&self, app: Code<'_>, input: Bytes, fuel: u64) -> Result<Outcome, Error> {
        let (host, bytes) = self.resolve(app).await?;
        let component = self.compile(bytes).await?;

        self.executor
            .execute(Context { input, host }, &component, fuel)
            .await
            .map_err(Into::into)
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
