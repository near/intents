mod cache;
mod code;
mod hashed_code;
mod resolver;

pub use self::resolver::Resolver;
pub use self::{cache::*, code::*, hashed_code::*};

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
    runtime_cache: Option<Cache<[u8; 32], Component>>,
}

impl Outlayer {
    pub const fn new(
        resolver: Resolver,
        executor: Executor,
        cache: Option<Cache<[u8; 32], Component>>,
    ) -> Self {
        Self {
            resolver,
            executor,
            runtime_cache: cache,
        }
    }

    async fn resolve(&self, app: Code<'_>) -> Result<(HostContext<'static>, HashedCode), Error> {
        match app {
            Code::Ref(code_ref) => Ok((
                HostContext {
                    app_id: code_ref.app_id(),
                },
                self.resolver.resolve_code(code_ref).await?,
            )),
            Code::Inline { code } => {
                let hashed = HashedCode::new(code);
                Ok((
                    HostContext {
                        app_id: AppCodeUrl::from_code(hashed.clone()).immutable_app_id(),
                    },
                    hashed,
                ))
            }
        }
    }

    async fn compile(&self, code: HashedCode) -> Result<Component, Error> {
        let do_compile = |bytes: Bytes| async {
            let compiler = self.executor.compiler();
            tokio::task::spawn_blocking(move || compiler.compile(bytes))
                .await
                .map_err(|e| anyhow::anyhow!("compile panicked: {e}"))?
        };

        if let Some(cache) = &self.runtime_cache {
            let hash = *code.hash();
            cache.try_get_with(hash, do_compile(code.bytes())).await
        } else {
            do_compile(code.bytes()).await.map_err(Arc::new)
        }
        .map_err(Error::Compile)
    }

    pub async fn execute(&self, app: Code<'_>, input: Bytes, fuel: u64) -> Result<Outcome, Error> {
        let (host, code) = self.resolve(app).await?;
        let component = self.compile(code).await?;

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
