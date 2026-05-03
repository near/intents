mod app;
mod error;
mod resolver;

use bytes::Bytes;
use defuse_outlayer_executor::{Context, Executor, HostContext, Outcome};

pub use self::{app::*, error::*, resolver::*};

pub struct Outlayer<R> {
    resolver: R,
    executor: Executor,
}

impl<R> Outlayer<R>
where
    R: Resolver,
{
    pub async fn execute(
        &self,
        app: App,
        input: Bytes,
        fuel: u64,
    ) -> Result<Outcome, Error<R::Error>> {
        let app_id = app.app_id();
        let wasm = match app {
            App::Inline { wasm } => wasm,
            App::AppId(app_id) => self
                .resolver
                .resolve_wasm(app_id)
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
