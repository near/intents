mod cache;
mod code;
mod resolver;

pub use self::resolver::Resolver;
pub use self::{cache::*, code::*};

use std::sync::Arc;

use bytes::Bytes;
use defuse_outlayer_executor::{
    self as executor, CompileError, Component, Context, Executor, HostContext, Outcome,
};
use moka::future::Cache;
use sha2::{Digest, Sha256};

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

    async fn resolve(&self, app: Code<'_>) -> Result<(HostContext<'static>, Bytes), Error> {
        match app {
            Code::Ref(CodeRef::AppId(app_id)) => {
                let app_id = app_id.into_owned();
                let url = self.resolver.resolve_app_id(app_id.as_ref()).await?;
                let bytes = self.resolver.fetch_and_verify(&url).await?;
                Ok((HostContext { app_id }, bytes))
            }
            Code::Ref(CodeRef::Url(app_code_url)) => {
                let app_id = app_code_url.immutable_app_id();
                let bytes = self.resolver.fetch_and_verify(&app_code_url).await?;
                Ok((HostContext { app_id }, bytes))
            }
            Code::Inline { code } => {
                let app_code_url: AppCodeUrl = code.clone().into();
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
        let code_hash = Sha256::digest(&code).into();

        let do_compile = |code| async {
            let compiler = self.executor.compiler();
            tokio::task::spawn_blocking(move || compiler.compile(code))
                .await
                .map_err(|e| CompileError(anyhow::anyhow!("compile panicked: {e}")))?
                .map_err(CompileError)
        };

        if let Some(cache) = &self.runtime_cache {
            cache
                .try_get_with(code_hash, do_compile(code))
                .await
                .map_err(Error::Compile)
        } else {
            do_compile(code).await.map_err(|e| Error::Compile(Arc::new(e)))
        }
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
    Compile(Arc<CompileError>),
    #[error(transparent)]
    Execute(#[from] executor::Error),
}
