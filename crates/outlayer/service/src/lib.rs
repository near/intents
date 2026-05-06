mod builder;
mod cache;
mod code;
mod hashed_code;
mod resolver;

pub use self::resolver::Resolver;
pub use self::{builder::*, cache::*, code::*, hashed_code::*};

use std::sync::Arc;

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
}

impl Outlayer {
    pub fn builder() -> OutlayerBuilder {
        OutlayerBuilder::default()
    }

    async fn resolve(&self, app: Code<'_>) -> Result<(AppId<'static>, HashedCode), Error> {
        match app {
            Code::Ref(code_ref) => {
                let app_id = code_ref.app_id();
                let hashed = self.resolver.resolve_code(code_ref).await?;
                Ok((app_id, hashed))
            }
            Code::Inline { code } => {
                let hashed = HashedCode::new(code);
                let app_id = AppCodeUrl::from_code(hashed.clone()).immutable_app_id();
                Ok((app_id, hashed))
            }
        }
    }

    async fn compile(&self, code: HashedCode) -> Result<Component, Error> {
        self.runtime_cache
            .try_get_with(*code.hash(), async {
                let compiler = self.executor.compiler();
                let bytes = code.bytes();
                tokio::task::spawn_blocking(move || compiler.compile(bytes))
                    .await
                    .map_err(|e| anyhow::anyhow!("compile panicked: {e}"))?
            })
            .await
            .map_err(Error::Compile)
    }

    pub async fn execute(&self, app: Code<'_>, input: Bytes, fuel: u64) -> Result<Outcome, Error> {
        let (app_id, code) = self.resolve(app).await?;
        let component = self.compile(code).await?;

        self.executor
            .execute(
                Context {
                    input,
                    host: HostContext { app_id },
                },
                &component,
                fuel,
            )
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
