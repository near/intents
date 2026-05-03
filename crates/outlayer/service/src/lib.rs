mod code;
mod resolver;

pub use self::code::*;

use bytes::Bytes;
use defuse_outlayer_executor::{self as executor, Context, Executor, HostContext, Outcome};

use crate::resolver::Resolver;

pub struct Outlayer {
    resolver: Resolver,
    executor: Executor,
}

impl Outlayer {
    pub async fn execute(&self, app: Code<'_>, input: Bytes, fuel: u64) -> Result<Outcome, Error> {
        let app_id = app.app_id();

        let wasm = match app {
            Code::Inline { code: wasm } => wasm,
            Code::Ref(app_id) => self
                .resolver
                .resolve_code(app_id)
                .await
                .map_err(Error::Resolve)?,
        };

        self.executor
            .execute(
                Context {
                    input,
                    host: HostContext { app_id },
                },
                wasm,
                fuel,
            )
            .await
            .map_err(Error::Execute)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("resolve: {0}")]
    Resolve(resolver::Error),
    #[error(transparent)]
    Execute(executor::Error),
}
